#![no_std]

//! Time-windowed transfer-limits compliance module — Stellar port of T-REX
//! [`TimeTransfersLimitsModule.sol`][trex-src].
//!
//! Limits transfer volume within configurable time windows, tracking counters
//! per **identity** (not per wallet) — matching the EVM module's
//! `usersCounters[compliance][identity]`.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                        |
//! |------------------------|-----------------|--------------------------------------------------|
//! | `moduleCheck`          | `can_transfer`  | Validate transfer against all configured windows |
//! | _(same)_               | `can_create`   | Always true (mints don't count toward limits)    |
//! | `moduleTransferAction` | `on_transfer`   | Resolve sender identity, increase counters       |
//! | `moduleMintAction`     | `on_created`    | No-op                                            |
//! | `moduleBurnAction`     | `on_destroyed`  | No-op                                            |
//!
//! ## Required hooks
//!
//! `CanTransfer`, `Transferred`
//!
//! Call `verify_hook_wiring()` after wiring to arm the module. The
//! `can_transfer` hook panics if the module is not armed — this prevents
//! silent misconfiguration where a missing `Transferred` hook would mean
//! counters never increment and all rate limits are bypassed.
//!
//! ## Differences from T-REX
//!
//! - T-REX `moduleCheck` returns true for token agents (`_isTokenAgent`). In
//!   Stellar, agent permissions are handled by the token's RBAC layer before
//!   compliance hooks fire, so the bypass is not replicated here.
//! - Limits and counters are token-scoped. Window reset behavior is explicit
//!   and deterministic (`timer <= now` starts a fresh bucket).
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeTransfersLimitsModule.sol

pub mod storage;

use soroban_sdk::{contract, contractevent, contractimpl, panic_with_error, vec, Address, Env, Vec};
use stellar_compliance_common::{
    checked_add_i128, get_compliance_address, get_irs_client, hooks_verified, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address, set_irs_address,
    verify_required_hooks, ModuleError,
};
use stellar_tokens::rwa::compliance::{ComplianceHook, ComplianceModule};

pub use storage::{Limit, TransferCounter};
use storage::{get_counter, get_limits, set_counter, set_limits};

/// Maximum number of distinct time-window limits per token.
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

/// Rate-limits transfer volume per identity within configurable time windows.
#[contract]
pub struct TimeTransfersLimitsModule;

#[contractimpl]
impl TimeTransfersLimitsModule {
    /// Configures the IRS address used for identity lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    /// Adds or updates a time-window limit for `token`. Replaces an
    /// existing entry with the same `limit_time`.
    /// T-REX equivalent: `setTimeTransferLimit(_limit)`.
    pub fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit) {
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
                panic_with_error!(e, ModuleError::MathOverflow);
            }
            limits.push_back(limit.clone());
        }

        set_limits(e, &token, &limits);
        TimeTransferLimitUpdated { token, limit }.publish(e);
    }

    /// Adds or updates multiple time-window limits in a single call.
    pub fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>) {
        require_compliance_auth(e);
        for limit in limits.iter() {
            Self::set_time_transfer_limit(e, token.clone(), limit);
        }
    }

    /// Removes the limit entry matching `limit_time`. Panics if not found.
    pub fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64) {
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
            panic_with_error!(e, ModuleError::MissingLimit);
        }

        set_limits(e, &token, &limits);
        TimeTransferLimitRemoved { token, limit_time }.publish(e);
    }

    /// Removes multiple time transfer limits in a single call.
    /// Mirrors T-REX `batchRemoveTimeTransferLimit`.
    pub fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>) {
        require_compliance_auth(e);
        for lt in limit_times.iter() {
            Self::remove_time_transfer_limit(e, token.clone(), lt);
        }
    }

    /// Returns all configured time-window limits for `token`.
    pub fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<Limit> {
        get_limits(e, &token)
    }

    /// Returns the compliance hooks this module must be registered on.
    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![e, ComplianceHook::CanTransfer, ComplianceHook::Transferred]
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

    fn is_counter_finished(e: &Env, token: &Address, identity: &Address, limit_time: u64) -> bool {
        let counter = get_counter(e, token, identity, limit_time);
        counter.timer <= e.ledger().timestamp()
    }

    fn reset_counter_if_needed(e: &Env, token: &Address, identity: &Address, limit_time: u64) {
        if Self::is_counter_finished(e, token, identity, limit_time) {
            let counter = TransferCounter {
                value: 0,
                timer: e.ledger().timestamp().saturating_add(limit_time),
            };
            set_counter(e, token, identity, limit_time, &counter);
        }
    }

    fn increase_counters(e: &Env, token: &Address, identity: &Address, value: i128) {
        let limits = get_limits(e, token);
        for limit in limits.iter() {
            Self::reset_counter_if_needed(e, token, identity, limit.limit_time);
            let mut counter = get_counter(e, token, identity, limit.limit_time);
            counter.value = checked_add_i128(e, counter.value, value);
            set_counter(e, token, identity, limit.limit_time, &counter);
        }
    }
}

#[contractimpl]
impl ComplianceModule for TimeTransfersLimitsModule {
    /// Resolves sender identity and increments per-window transfer counters.
    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        Self::increase_counters(e, &token, &from_id, amount);
    }

    /// No-op — mints are exempt from rate limits.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — burns are exempt from rate limits.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// T-REX `moduleCheck` also bypasses limits for token agents via
    /// `_isTokenAgent`. In Stellar, agent-level permissions are handled by
    /// the token contract's RBAC layer before compliance hooks fire, so
    /// the bypass is not replicated here.
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

            if !Self::is_counter_finished(e, &token, &from_id, limit.limit_time) {
                let counter = get_counter(e, &token, &from_id, limit.limit_time);
                if checked_add_i128(e, counter.value, amount) > limit.limit_value {
                    return false;
                }
            }
        }

        true
    }

    /// Always returns `true` — mints are exempt from rate limits.
    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "TimeTransfersLimitsModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
