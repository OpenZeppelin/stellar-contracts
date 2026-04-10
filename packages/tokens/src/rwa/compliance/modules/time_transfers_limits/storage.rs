use soroban_sdk::{contracttype, panic_with_error, vec, Address, Env, Vec};

use super::{TimeTransferLimitRemoved, TimeTransferLimitUpdated, MAX_LIMITS_PER_TOKEN};
use crate::rwa::compliance::{
    modules::{
        storage::{
            add_i128_or_panic, get_irs_client, hooks_verified, require_non_negative_amount,
            set_irs_address, verify_required_hooks,
        },
        ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    ComplianceHook,
};

/// A single time-window limit: `limit_value` tokens may be transferred
/// within a rolling window of `limit_time` seconds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Limit {
    pub limit_time: u64,
    pub limit_value: i128,
}

/// Tracks cumulative transfer volume for one identity within one window.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferCounter {
    pub value: i128,
    pub timer: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum TimeTransfersLimitsStorageKey {
    /// Per-token list of configured time-window limits.
    Limits(Address),
    /// Counter keyed by (token, identity, window_seconds).
    Counter(Address, Address, u64),
}

// ################## RAW STORAGE ##################

/// Returns the list of time-window limits for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_limits(e: &Env, token: &Address) -> Vec<Limit> {
    let key = TimeTransfersLimitsStorageKey::Limits(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &Vec<Limit>| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_else(|| Vec::new(e))
}

/// Persists the list of time-window limits for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limits` - The updated limits list.
pub fn set_limits(e: &Env, token: &Address, limits: &Vec<Limit>) {
    let key = TimeTransfersLimitsStorageKey::Limits(token.clone());
    e.storage().persistent().set(&key, limits);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the transfer counter for a given identity and time window.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
/// * `limit_time` - The time-window duration in seconds.
pub fn get_counter(
    e: &Env,
    token: &Address,
    identity: &Address,
    limit_time: u64,
) -> TransferCounter {
    let key = TimeTransfersLimitsStorageKey::Counter(token.clone(), identity.clone(), limit_time);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &TransferCounter| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or(TransferCounter { value: 0, timer: 0 })
}

/// Persists the transfer counter for a given identity and time window.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
/// * `limit_time` - The time-window duration in seconds.
/// * `counter` - The updated counter value.
pub fn set_counter(
    e: &Env,
    token: &Address,
    identity: &Address,
    limit_time: u64,
    counter: &TransferCounter,
) {
    let key = TimeTransfersLimitsStorageKey::Counter(token.clone(), identity.clone(), limit_time);
    e.storage().persistent().set(&key, counter);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

// ################## HELPERS ##################

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
        counter.value = add_i128_or_panic(e, counter.value, value);
        set_counter(e, token, identity, limit.limit_time, &counter);
    }
}

// ################## ACTIONS ##################

/// Configures the identity registry storage address for a token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `irs` - The identity registry storage address.
pub fn configure_irs(e: &Env, token: &Address, irs: &Address) {
    set_irs_address(e, token, irs);
}

/// Sets or updates a time-window transfer limit for `token` and emits
/// [`TimeTransferLimitUpdated`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit` - The limit to set.
pub fn set_time_transfer_limit(e: &Env, token: &Address, limit: &Limit) {
    assert!(limit.limit_time > 0, "limit_time must be greater than zero");
    require_non_negative_amount(e, limit.limit_value);
    let mut limits = get_limits(e, token);

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
            panic_with_error!(e, ComplianceModuleError::TooManyLimits);
        }
        limits.push_back(limit.clone());
    }

    set_limits(e, token, &limits);
    TimeTransferLimitUpdated { token: token.clone(), limit: limit.clone() }.publish(e);
}

/// Sets or updates multiple time-window transfer limits in a single call.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limits` - The limits to set.
pub fn batch_set_time_transfer_limit(e: &Env, token: &Address, limits: &Vec<Limit>) {
    for limit in limits.iter() {
        set_time_transfer_limit(e, token, &limit);
    }
}

/// Removes a time-window transfer limit and emits
/// [`TimeTransferLimitRemoved`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit_time` - The time-window to remove.
pub fn remove_time_transfer_limit(e: &Env, token: &Address, limit_time: u64) {
    let mut limits = get_limits(e, token);

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

    set_limits(e, token, &limits);
    TimeTransferLimitRemoved { token: token.clone(), limit_time }.publish(e);
}

/// Removes multiple time-window transfer limits in a single call.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit_times` - The time-windows to remove.
pub fn batch_remove_time_transfer_limit(e: &Env, token: &Address, limit_times: &Vec<u64>) {
    for lt in limit_times.iter() {
        remove_time_transfer_limit(e, token, lt);
    }
}

/// Pre-seeds a transfer counter for a given identity and time window.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
/// * `limit_time` - The time-window duration in seconds.
/// * `counter` - The counter value to set.
pub fn pre_set_transfer_counter(
    e: &Env,
    token: &Address,
    identity: &Address,
    limit_time: u64,
    counter: &TransferCounter,
) {
    require_non_negative_amount(e, counter.value);
    assert!(limit_time > 0, "limit_time must be greater than zero");

    let mut found = false;
    for limit in get_limits(e, token).iter() {
        if limit.limit_time == limit_time {
            found = true;
            break;
        }
    }

    if !found {
        panic_with_error!(e, ComplianceModuleError::MissingLimit);
    }

    set_counter(e, token, identity, limit_time, counter);
}

// ################## HOOK WIRING ##################

/// Returns the set of compliance hooks this module requires.
pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
    vec![e, ComplianceHook::CanTransfer, ComplianceHook::Transferred]
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on all required hooks.
pub fn verify_hook_wiring(e: &Env) {
    verify_required_hooks(e, required_hooks(e));
}

// ################## COMPLIANCE HOOKS ##################

/// Resolves the sender's identity and increments transfer counters.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
pub fn on_transfer(e: &Env, from: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let irs = get_irs_client(e, token);
    let from_id = irs.stored_identity(from);
    increase_counters(e, token, &from_id, amount);
}

/// Checks whether a transfer is within the configured time-window limits.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
///
/// # Returns
///
/// `true` if the transfer does not exceed any limit.
pub fn can_transfer(e: &Env, from: &Address, amount: i128, token: &Address) -> bool {
    assert!(
        hooks_verified(e),
        "TimeTransfersLimitsModule: not armed — call verify_hook_wiring() after wiring hooks \
         [CanTransfer, Transferred]"
    );
    if amount < 0 {
        return false;
    }
    let irs = get_irs_client(e, token);
    let from_id = irs.stored_identity(from);
    let limits = get_limits(e, token);

    for limit in limits.iter() {
        if amount > limit.limit_value {
            return false;
        }

        if !is_counter_finished(e, token, &from_id, limit.limit_time) {
            let counter = get_counter(e, token, &from_id, limit.limit_time);
            if add_i128_or_panic(e, counter.value, amount) > limit.limit_value {
                return false;
            }
        }
    }

    true
}
