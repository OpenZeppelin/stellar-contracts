use soroban_sdk::{contracttype, Address, Env, Vec};

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

pub fn get_limits(e: &Env, token: &Address) -> Vec<Limit> {
    e.storage()
        .persistent()
        .get(&TimeTransfersLimitsStorageKey::Limits(token.clone()))
        .unwrap_or_else(|| Vec::new(e))
}

pub fn set_limits(e: &Env, token: &Address, limits: &Vec<Limit>) {
    e.storage()
        .persistent()
        .set(&TimeTransfersLimitsStorageKey::Limits(token.clone()), limits);
}

pub fn get_counter(e: &Env, token: &Address, identity: &Address, limit_time: u64) -> TransferCounter {
    e.storage()
        .persistent()
        .get(&TimeTransfersLimitsStorageKey::Counter(
            token.clone(),
            identity.clone(),
            limit_time,
        ))
        .unwrap_or(TransferCounter { value: 0, timer: 0 })
}

pub fn set_counter(
    e: &Env,
    token: &Address,
    identity: &Address,
    limit_time: u64,
    counter: &TransferCounter,
) {
    e.storage().persistent().set(
        &TimeTransfersLimitsStorageKey::Counter(token.clone(), identity.clone(), limit_time),
        counter,
    );
}
