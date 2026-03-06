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
//! ## Required hooks
//!
//! `CanTransfer`, `CanCreate`, `Transferred`, `Created`, `Destroyed`
//!
//! Call `verify_hook_wiring()` after wiring to arm the module. The
//! `can_transfer` / `can_create` hooks panic if the module is not armed —
//! this prevents silent misconfiguration where missing hooks would cause the
//! internal identity-balance to drift.
//!
//! ## Differences from T-REX
//!
//! - No `_compliancePresetStatus` / `presetCompleted()` lifecycle tracking.
//!   Stellar does not enforce preset ordering before module binding.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

pub mod storage;

use soroban_sdk::{contract, contractevent, contractimpl, vec, Address, Env, Vec};
use stellar_compliance_common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, get_irs_client, hooks_verified,
    module_name, require_compliance_auth, require_non_negative_amount, set_compliance_address,
    set_irs_address, verify_required_hooks,
};
use stellar_tokens::rwa::compliance::{ComplianceHook, ComplianceModule};

use storage::{get_id_balance, get_max_balance, set_id_balance, set_max_balance};

/// Emitted when a token's per-identity balance cap is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBalanceSet {
    #[topic]
    pub token: Address,
    pub max_balance: i128,
}

/// Emitted when an identity balance is pre-seeded via `pre_set_module_state`.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IDBalancePreSet {
    #[topic]
    pub token: Address,
    pub identity: Address,
    pub balance: i128,
}

/// Enforces a per-identity balance cap across all wallets belonging
/// to the same on-chain identity.
#[contract]
pub struct MaxBalanceModule;

#[contractimpl]
impl MaxBalanceModule {
    /// Configures the IRS address used for identity lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    /// Configures the per-identity balance cap for `token`.
    /// T-REX equivalent: `setMaxBalance(_newMaxBalance)`.
    pub fn set_max_balance(e: &Env, token: Address, max: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, max);
        set_max_balance(e, &token, max);
        MaxBalanceSet { token, max_balance: max }.publish(e);
    }

    /// Bootstrap existing investor state. Takes an **identity** address
    /// directly (not a wallet), matching T-REX `preSetModuleState`.
    pub fn pre_set_module_state(e: &Env, token: Address, identity: Address, balance: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, balance);
        set_id_balance(e, &token, &identity, balance);
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
            set_id_balance(e, &token, &id, bal);
            IDBalancePreSet { token: token.clone(), identity: id, balance: bal }.publish(e);
        }
    }

    /// Returns the module-tracked balance for an **identity**.
    pub fn get_investor_balance(e: &Env, token: Address, identity: Address) -> i128 {
        get_id_balance(e, &token, &identity)
    }

    /// Returns the compliance hooks this module must be registered on.
    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![
            e,
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ]
    }

    /// Arms the module by verifying all required hooks are wired.
    ///
    /// Must be called **once after wiring** (outside the hook chain) because
    /// it cross-calls the compliance contract. Panics with a message naming
    /// the first missing hook. Caches the result so that subsequent `can_*`
    /// calls only check a boolean flag.
    pub fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
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

        let from_balance = get_id_balance(e, &token, &from_id);
        let to_balance = get_id_balance(e, &token, &to_id);
        let new_to_balance = checked_add_i128(e, to_balance, amount);

        let max = get_max_balance(e, &token);
        assert!(
            max == 0 || new_to_balance <= max,
            "MaxBalanceModule: recipient identity balance exceeds max"
        );

        set_id_balance(e, &token, &from_id, checked_sub_i128(e, from_balance, amount));
        set_id_balance(e, &token, &to_id, new_to_balance);
    }

    /// Mirrors T-REX `moduleMintAction`: updates identity balance and reverts
    /// if the recipient identity exceeds the configured max
    /// (belt-and-suspenders invariant matching T-REX's post-mint check).
    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let to_id = irs.stored_identity(&to);

        let current = get_id_balance(e, &token, &to_id);
        let new_balance = checked_add_i128(e, current, amount);

        let max = get_max_balance(e, &token);
        assert!(
            max == 0 || new_balance <= max,
            "MaxBalanceModule: recipient identity balance exceeds max after mint"
        );

        set_id_balance(e, &token, &to_id, new_balance);
    }

    /// Decrements the sender identity's tracked balance on burn.
    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);

        let current = get_id_balance(e, &token, &from_id);
        set_id_balance(e, &token, &from_id, checked_sub_i128(e, current, amount));
    }

    /// Returns `true` if recipient identity balance + amount stays within cap.
    /// Same-identity transfers (wallets sharing one identity) are always
    /// allowed since they don't change the net identity balance — matching the
    /// short-circuit in `on_transfer`.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        assert!(
            hooks_verified(e),
            "MaxBalanceModule: not armed — call verify_hook_wiring() after wiring hooks \
             [CanTransfer, CanCreate, Transferred, Created, Destroyed]"
        );
        if amount < 0 {
            return false;
        }
        let max = get_max_balance(e, &token);
        if max == 0 || amount > max {
            return max == 0;
        }

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        let to_id = irs.stored_identity(&to);

        if from_id == to_id {
            return true;
        }

        let to_balance = get_id_balance(e, &token, &to_id);
        checked_add_i128(e, to_balance, amount) <= max
    }

    /// Delegates to `can_transfer` — mints are subject to the same cap.
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
