#![no_std]

//! Max balance compliance module — Stellar port of T-REX
//! [`MaxBalanceModule.sol`][trex-src].
//!
//! Tracks effective balances per **identity** (not per wallet), enforcing a
//! per-token cap. Multiple wallets belonging to the same identity share one
//! aggregate balance — matching the EVM module's
//! `_IDBalance[compliance][identity]` accounting.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                          |
//! |------------------------|-----------------|----------------------------------------------------|
//! | `moduleCheck`          | `can_transfer`  | Pre-check recipient identity balance + amount ≤ max |
//! | _(same)_               | `can_create`    | Delegates to `can_transfer`                        |
//! | `moduleTransferAction` | `on_transfer`   | Update identity balances, revert if exceeds max    |
//! | `moduleMintAction`     | `on_created`    | Update identity balance, revert if exceeds max     |
//! | `moduleBurnAction`     | `on_destroyed`  | Decrease identity balance                          |
//!
//! ## Identity Resolution
//!
//! Wallet-to-identity mapping is resolved cross-contract from the Identity
//! Registry Storage at every hook invocation — matching the T-REX
//! `_getIdentity(compliance, userAddress)` pattern.
//!
//! ## Differences from T-REX
//!
//! - No `_compliancePresetStatus` / `presetCompleted()` lifecycle tracking.
//!   Stellar does not enforce preset ordering before module binding.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env, Vec};

use stellar_tokens::rwa::compliance::ComplianceModule;

use stellar_compliance_common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, get_irs_client, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address, set_irs_address,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    MaxBalance(Address),
    /// Balance keyed by (token, identity) — not by wallet.
    IDBalance(Address, Address),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBalanceSet {
    #[topic]
    pub token: Address,
    pub max_balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IDBalancePreSet {
    #[topic]
    pub token: Address,
    pub identity: Address,
    pub balance: i128,
}

#[contract]
pub struct MaxBalanceModule;

#[contractimpl]
impl MaxBalanceModule {
    /// Configures the IRS address used for identity lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    pub fn set_max_balance(e: &Env, token: Address, max_balance: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, max_balance);
        e.storage()
            .persistent()
            .set(&DataKey::MaxBalance(token.clone()), &max_balance);
        MaxBalanceSet { token, max_balance }.publish(e);
    }

    /// Bootstrap existing investor state. Takes an **identity** address
    /// directly (not a wallet), matching T-REX `preSetModuleState`.
    pub fn pre_set_module_state(e: &Env, token: Address, identity: Address, balance: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, balance);
        e.storage()
            .persistent()
            .set(&DataKey::IDBalance(token.clone(), identity.clone()), &balance);
        IDBalancePreSet { token, identity, balance }.publish(e);
    }

    /// Bootstrap multiple existing investor states in a single call.
    /// Mirrors T-REX `batchPreSetModuleState`.
    pub fn batch_pre_set_module_state(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    ) {
        require_compliance_auth(e);
        assert!(
            identities.len() == balances.len(),
            "MaxBalanceModule: identities and balances length mismatch"
        );
        for i in 0..identities.len() {
            let id = identities.get(i).unwrap();
            let bal = balances.get(i).unwrap();
            require_non_negative_amount(e, bal);
            e.storage()
                .persistent()
                .set(&DataKey::IDBalance(token.clone(), id.clone()), &bal);
            IDBalancePreSet { token: token.clone(), identity: id, balance: bal }.publish(e);
        }
    }

    /// Returns the module-tracked balance for an **identity**.
    pub fn get_investor_balance(e: &Env, token: Address, identity: Address) -> i128 {
        e.storage()
            .persistent()
            .get(&DataKey::IDBalance(token, identity))
            .unwrap_or_default()
    }
}

#[contractimpl]
impl ComplianceModule for MaxBalanceModule {
    /// Mirrors T-REX `moduleTransferAction`: updates identity balances and
    /// reverts if the recipient identity exceeds the configured max
    /// (belt-and-suspenders invariant matching T-REX's post-transfer check).
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        let to_id = irs.stored_identity(&to);

        if from_id == to_id {
            return;
        }

        let from_key = DataKey::IDBalance(token.clone(), from_id);
        let to_key = DataKey::IDBalance(token.clone(), to_id);

        let from_balance: i128 = e.storage().persistent().get(&from_key).unwrap_or_default();
        let to_balance: i128 = e.storage().persistent().get(&to_key).unwrap_or_default();

        let new_to_balance = checked_add_i128(e, to_balance, amount);

        let max: i128 = e
            .storage()
            .persistent()
            .get(&DataKey::MaxBalance(token))
            .unwrap_or_default();
        assert!(
            max == 0 || new_to_balance <= max,
            "MaxBalanceModule: recipient identity balance exceeds max"
        );

        e.storage()
            .persistent()
            .set(&from_key, &checked_sub_i128(e, from_balance, amount));
        e.storage().persistent().set(&to_key, &new_to_balance);
    }

    /// Mirrors T-REX `moduleMintAction`: updates identity balance and reverts
    /// if the recipient identity exceeds the configured max (belt-and-suspenders
    /// invariant matching T-REX's post-mint check).
    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let to_id = irs.stored_identity(&to);

        let key = DataKey::IDBalance(token.clone(), to_id);
        let current: i128 = e.storage().persistent().get(&key).unwrap_or_default();
        let new_balance = checked_add_i128(e, current, amount);

        let max: i128 = e
            .storage()
            .persistent()
            .get(&DataKey::MaxBalance(token))
            .unwrap_or_default();
        assert!(
            max == 0 || new_balance <= max,
            "MaxBalanceModule: recipient identity balance exceeds max after mint"
        );

        e.storage().persistent().set(&key, &new_balance);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);

        let key = DataKey::IDBalance(token, from_id);
        let current: i128 = e.storage().persistent().get(&key).unwrap_or_default();
        e.storage().persistent().set(&key, &checked_sub_i128(e, current, amount));
    }

    fn can_transfer(e: &Env, _from: Address, to: Address, amount: i128, token: Address) -> bool {
        if amount < 0 {
            return false;
        }
        let max: i128 = e
            .storage()
            .persistent()
            .get(&DataKey::MaxBalance(token.clone()))
            .unwrap_or_default();
        if max == 0 || amount > max {
            return max == 0;
        }

        let irs = get_irs_client(e, &token);
        let to_id = irs.stored_identity(&to);

        let to_balance: i128 = e
            .storage()
            .persistent()
            .get(&DataKey::IDBalance(token, to_id))
            .unwrap_or_default();

        checked_add_i128(e, to_balance, amount) <= max
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        Self::can_transfer(e, to.clone(), to, amount, token)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "MaxBalanceModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
