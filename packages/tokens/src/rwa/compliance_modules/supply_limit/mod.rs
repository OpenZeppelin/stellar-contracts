//! Supply cap compliance module — Stellar port of T-REX
//! [`SupplyLimitModule.sol`][trex-src].
//!
//! Caps the total number of tokens that can be minted for a given token.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/SupplyLimitModule.sol

pub mod storage;

use soroban_sdk::{contractevent, contracttrait, vec, Address, Env, String, Vec};

use super::common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, hooks_verified, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address,
    verify_required_hooks,
};
use crate::rwa::compliance::ComplianceHook;
use storage::{
    get_internal_supply, get_supply_limit, get_supply_limit_or_panic, set_internal_supply,
    set_supply_limit,
};

/// Emitted when a token's supply cap is configured or changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyLimitSet {
    #[topic]
    pub token: Address,
    pub limit: i128,
}

#[contracttrait]
pub trait SupplyLimit {
    fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, limit);
        set_supply_limit(e, &token, limit);
        SupplyLimitSet { token, limit }.publish(e);
    }

    fn get_supply_limit(e: &Env, token: Address) -> i128 {
        get_supply_limit_or_panic(e, &token)
    }

    fn get_internal_supply(e: &Env, token: Address) -> i128 {
        get_internal_supply(e, &token)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![e, ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(e: &Env, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let current = get_internal_supply(e, &token);
        set_internal_supply(e, &token, checked_add_i128(e, current, amount));
    }

    fn on_destroyed(e: &Env, _from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let current = get_internal_supply(e, &token);
        set_internal_supply(e, &token, checked_sub_i128(e, current, amount));
    }

    fn can_transfer(
        _e: &Env,
        _from: Address,
        _to: Address,
        _amount: i128,
        _token: Address,
    ) -> bool {
        true
    }

    fn can_create(e: &Env, _to: Address, amount: i128, token: Address) -> bool {
        assert!(
            hooks_verified(e),
            "SupplyLimitModule: not armed — call verify_hook_wiring() after wiring hooks \
             [CanCreate, Created, Destroyed]"
        );
        if amount < 0 {
            return false;
        }
        let limit = get_supply_limit(e, &token);
        if limit == 0 {
            return true;
        }
        let supply = get_internal_supply(e, &token);
        checked_add_i128(e, supply, amount) <= limit
    }

    fn name(e: &Env) -> String {
        module_name(e, "SupplyLimitModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
