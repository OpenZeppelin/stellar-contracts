use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::rwa::compliance_modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

/// A single mint-created lock entry tracking the locked amount and its
/// release time. Mirrors T-REX `LockedTokens { amount, releaseTimestamp }`.
#[contracttype]
#[derive(Clone)]
pub struct LockedTokens {
    pub amount: i128,
    pub release_timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum InitialLockupStorageKey {
    /// Per-token lockup duration in seconds.
    LockupPeriod(Address),
    /// Per-(token, wallet) ordered list of individual lock entries.
    Locks(Address, Address),
    /// Per-(token, wallet) aggregate of all locked amounts.
    TotalLocked(Address, Address),
    /// Per-(token, wallet) balance mirror, updated via hooks to avoid
    /// re-entrant `token.balance()` calls.
    InternalBalance(Address, Address),
}

/// Returns the lockup period (in seconds) for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_lockup_period(e: &Env, token: &Address) -> u64 {
    let key = InitialLockupStorageKey::LockupPeriod(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &u64| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the lockup period (in seconds) for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `seconds` - The lockup duration in seconds.
pub fn set_lockup_period(e: &Env, token: &Address, seconds: u64) {
    let key = InitialLockupStorageKey::LockupPeriod(token.clone());
    e.storage().persistent().set(&key, &seconds);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the lock entries for `wallet` on `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
pub fn get_locks(e: &Env, token: &Address, wallet: &Address) -> Vec<LockedTokens> {
    let key = InitialLockupStorageKey::Locks(token.clone(), wallet.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &Vec<LockedTokens>| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_else(|| Vec::new(e))
}

/// Persists the lock entries for `wallet` on `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `locks` - The updated lock entries.
pub fn set_locks(e: &Env, token: &Address, wallet: &Address, locks: &Vec<LockedTokens>) {
    let key = InitialLockupStorageKey::Locks(token.clone(), wallet.clone());
    e.storage().persistent().set(&key, locks);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the total locked amount for `wallet` on `token`, or `0`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
pub fn get_total_locked(e: &Env, token: &Address, wallet: &Address) -> i128 {
    let key = InitialLockupStorageKey::TotalLocked(token.clone(), wallet.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the total locked amount for `wallet` on `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `amount` - The new total locked amount.
pub fn set_total_locked(e: &Env, token: &Address, wallet: &Address, amount: i128) {
    let key = InitialLockupStorageKey::TotalLocked(token.clone(), wallet.clone());
    e.storage().persistent().set(&key, &amount);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the internal balance for `wallet` on `token`, or `0`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
pub fn get_internal_balance(e: &Env, token: &Address, wallet: &Address) -> i128 {
    let key = InitialLockupStorageKey::InternalBalance(token.clone(), wallet.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the internal balance for `wallet` on `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `balance` - The new balance value.
pub fn set_internal_balance(e: &Env, token: &Address, wallet: &Address, balance: i128) {
    let key = InitialLockupStorageKey::InternalBalance(token.clone(), wallet.clone());
    e.storage().persistent().set(&key, &balance);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}
