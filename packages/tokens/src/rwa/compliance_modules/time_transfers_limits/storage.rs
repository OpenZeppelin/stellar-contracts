use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::rwa::compliance_modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

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
