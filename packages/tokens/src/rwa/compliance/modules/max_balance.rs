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

use crate::rwa::compliance::ComplianceModule;

use super::common::{
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

#[cfg(test)]
mod test {
    use soroban_sdk::{contract, testutils::Address as _, Address, Env};

    use crate::rwa::compliance::ComplianceModuleClient;

    use super::MaxBalanceModule;
    use crate::rwa::compliance::modules::test_utils::{MockIRS, MockIRSClient};

    #[contract]
    struct MockCompliance;

    #[test]
    fn max_balance_blocks_over_limit() {
        let e = Env::default();
        e.mock_all_auths();

        let module = e.register(MaxBalanceModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);

        let module_client = super::MaxBalanceModuleClient::new(&e, &module);
        let irs_helper = MockIRSClient::new(&e, &irs);

        // MockIRS defaults identity == wallet, so pre_set_module_state
        // takes the wallet address directly here.
        irs_helper.mock_set_identity(&from, &from);
        irs_helper.mock_set_identity(&to, &to);

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.set_max_balance(&token, &100);
            module_client.pre_set_module_state(&token, &from, &80);
            module_client.pre_set_module_state(&token, &to, &30);
        });

        assert!(!client.can_transfer(&from, &to, &71, &token));
        assert!(client.can_transfer(&from, &to, &70, &token));
    }

    #[test]
    fn on_transfer_keeps_balances_consistent() {
        let e = Env::default();
        e.mock_all_auths();

        let module = e.register(MaxBalanceModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);

        let module_client = super::MaxBalanceModuleClient::new(&e, &module);

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.pre_set_module_state(&token, &from, &10);
            client.on_transfer(&from, &to, &9, &token);
        });

        assert_eq!(module_client.get_investor_balance(&token, &from), 1);
        assert_eq!(module_client.get_investor_balance(&token, &to), 9);
    }

    #[test]
    fn shared_identity_aggregates_balance() {
        let e = Env::default();
        e.mock_all_auths();

        let module = e.register(MaxBalanceModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());

        let wallet_a = Address::generate(&e);
        let wallet_b = Address::generate(&e);
        let shared_id = Address::generate(&e);
        let sender = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);

        let module_client = super::MaxBalanceModuleClient::new(&e, &module);
        let irs_helper = MockIRSClient::new(&e, &irs);

        irs_helper.mock_set_identity(&wallet_a, &shared_id);
        irs_helper.mock_set_identity(&wallet_b, &shared_id);

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.set_max_balance(&token, &100);
            module_client.pre_set_module_state(&token, &sender, &200);
        });

        e.as_contract(&compliance, || {
            client.on_transfer(&sender, &wallet_a, &60, &token);
        });
        assert_eq!(module_client.get_investor_balance(&token, &shared_id), 60);

        assert!(!client.can_transfer(&sender, &wallet_b, &41, &token));
        assert!(client.can_transfer(&sender, &wallet_b, &40, &token));
    }
}
