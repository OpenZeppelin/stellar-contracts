//! Time-windowed transfer-limits compliance module — Stellar port of T-REX
//! [`TimeTransfersLimitsModule.sol`][trex-src].
//!
//! Limits transfer volume within configurable time windows, tracking counters
//! per **identity** (not per wallet).
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeTransfersLimitsModule.sol

pub mod storage;

use soroban_sdk::{contractevent, contracttrait, panic_with_error, vec, Address, Env, String, Vec};
use storage::{get_counter, get_limits, set_counter, set_limits};
pub use storage::{Limit, TransferCounter};

use super::common::{
    checked_add_i128, get_compliance_address, get_irs_client, hooks_verified, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address, set_irs_address,
    verify_required_hooks,
};
use crate::rwa::{compliance::ComplianceHook, compliance_modules::ComplianceModuleError};

const MAX_LIMITS_PER_TOKEN: u32 = 4;

/// Emitted when a time-window limit is added or updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitUpdated {
    #[topic]
    pub token: Address,
    pub limit: Limit,
}

/// Emitted when a time-window limit is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitRemoved {
    #[topic]
    pub token: Address,
    pub limit_time: u64,
}

// ---------------------------------------------------------------------------
// Private helpers (not exposed as contract endpoints)
// ---------------------------------------------------------------------------

fn is_counter_finished(e: &Env, token: &Address, identity: &Address, limit_time: u64) -> bool {
    let counter = get_counter(e, token, identity, limit_time);
    counter.timer <= e.ledger().timestamp()
}

fn reset_counter_if_needed(e: &Env, token: &Address, identity: &Address, limit_time: u64) {
    if is_counter_finished(e, token, identity, limit_time) {
        let counter =
            TransferCounter { value: 0, timer: e.ledger().timestamp().saturating_add(limit_time) };
        set_counter(e, token, identity, limit_time, &counter);
    }
}

fn increase_counters(e: &Env, token: &Address, identity: &Address, value: i128) {
    let limits = get_limits(e, token);
    for limit in limits.iter() {
        reset_counter_if_needed(e, token, identity, limit.limit_time);
        let mut counter = get_counter(e, token, identity, limit.limit_time);
        counter.value = checked_add_i128(e, counter.value, value);
        set_counter(e, token, identity, limit.limit_time, &counter);
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[contracttrait]
pub trait TimeTransfersLimits {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit) {
        require_compliance_auth(e);
        assert!(limit.limit_time > 0, "limit_time must be greater than zero");
        require_non_negative_amount(e, limit.limit_value);
        let mut limits = get_limits(e, &token);

        let mut replaced = false;
        for i in 0..limits.len() {
            let current = limits.get(i).expect("limit exists");
            if current.limit_time == limit.limit_time {
                limits.set(i, limit.clone());
                replaced = true;
                break;
            }
        }

        if !replaced {
            if limits.len() >= MAX_LIMITS_PER_TOKEN {
                panic_with_error!(e, ComplianceModuleError::MathOverflow);
            }
            limits.push_back(limit.clone());
        }

        set_limits(e, &token, &limits);
        TimeTransferLimitUpdated { token, limit }.publish(e);
    }

    fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>) {
        require_compliance_auth(e);
        for limit in limits.iter() {
            Self::set_time_transfer_limit(e, token.clone(), limit);
        }
    }

    fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64) {
        require_compliance_auth(e);
        let mut limits = get_limits(e, &token);

        let mut found = false;
        for i in 0..limits.len() {
            let current = limits.get(i).expect("limit exists");
            if current.limit_time == limit_time {
                limits.remove(i);
                found = true;
                break;
            }
        }

        if !found {
            panic_with_error!(e, ComplianceModuleError::MissingLimit);
        }

        set_limits(e, &token, &limits);
        TimeTransferLimitRemoved { token, limit_time }.publish(e);
    }

    fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>) {
        require_compliance_auth(e);
        for lt in limit_times.iter() {
            Self::remove_time_transfer_limit(e, token.clone(), lt);
        }
    }

    fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<Limit> {
        get_limits(e, &token)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![e, ComplianceHook::CanTransfer, ComplianceHook::Transferred]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        increase_counters(e, &token, &from_id, amount);
    }

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        assert!(
            hooks_verified(e),
            "TimeTransfersLimitsModule: not armed — call verify_hook_wiring() after wiring hooks \
             [CanTransfer, Transferred]"
        );
        if amount < 0 {
            return false;
        }
        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        let limits = get_limits(e, &token);

        for limit in limits.iter() {
            if amount > limit.limit_value {
                return false;
            }

            if !is_counter_finished(e, &token, &from_id, limit.limit_time) {
                let counter = get_counter(e, &token, &from_id, limit.limit_time);
                if checked_add_i128(e, counter.value, amount) > limit.limit_value {
                    return false;
                }
            }
        }

        true
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> String {
        module_name(e, "TimeTransfersLimitsModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
