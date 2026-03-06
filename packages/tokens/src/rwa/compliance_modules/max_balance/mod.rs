//! Max balance compliance module — Stellar port of T-REX
//! [`MaxBalanceModule.sol`][trex-src].
//!
//! Tracks effective balances per **identity** (not per wallet), enforcing a
//! per-token cap.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

pub mod storage;

use soroban_sdk::{contractevent, contracttrait, vec, Address, Env, String, Vec};
use storage::{get_id_balance, get_max_balance, set_id_balance, set_max_balance};

use super::common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, get_irs_client, hooks_verified,
    module_name, require_compliance_auth, require_non_negative_amount, set_compliance_address,
    set_irs_address, verify_required_hooks,
};
use crate::rwa::compliance::ComplianceHook;

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

#[contracttrait]
pub trait MaxBalance {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn set_max_balance(e: &Env, token: Address, max: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, max);
        set_max_balance(e, &token, max);
        MaxBalanceSet { token, max_balance: max }.publish(e);
    }

    fn pre_set_module_state(e: &Env, token: Address, identity: Address, balance: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, balance);
        set_id_balance(e, &token, &identity, balance);
        IDBalancePreSet { token, identity, balance }.publish(e);
    }

    fn batch_pre_set_module_state(
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

    fn get_investor_balance(e: &Env, token: Address, identity: Address) -> i128 {
        get_id_balance(e, &token, &identity)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![
            e,
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

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

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);

        let current = get_id_balance(e, &token, &from_id);
        set_id_balance(e, &token, &from_id, checked_sub_i128(e, current, amount));
    }

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

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        Self::can_transfer(e, to.clone(), to, amount, token)
    }

    fn name(e: &Env) -> String {
        module_name(e, "MaxBalanceModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
