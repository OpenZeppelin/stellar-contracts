use soroban_sdk::{contracttype, Address, Env, Vec};

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
    /// Equivalent to T-REX `LockedDetails.totalLocked`.
    TotalLocked(Address, Address),
    /// Per-(token, wallet) balance mirror, updated via hooks to avoid
    /// re-entrant `token.balance()` calls.
    InternalBalance(Address, Address),
}

pub fn get_lockup_period(e: &Env, token: &Address) -> u64 {
    e.storage()
        .persistent()
        .get(&InitialLockupStorageKey::LockupPeriod(token.clone()))
        .unwrap_or_default()
}

pub fn set_lockup_period(e: &Env, token: &Address, seconds: u64) {
    e.storage()
        .persistent()
        .set(&InitialLockupStorageKey::LockupPeriod(token.clone()), &seconds);
}

pub fn get_locks(e: &Env, token: &Address, wallet: &Address) -> Vec<LockedTokens> {
    e.storage()
        .persistent()
        .get(&InitialLockupStorageKey::Locks(token.clone(), wallet.clone()))
        .unwrap_or_else(|| Vec::new(e))
}

pub fn set_locks(e: &Env, token: &Address, wallet: &Address, locks: &Vec<LockedTokens>) {
    e.storage()
        .persistent()
        .set(&InitialLockupStorageKey::Locks(token.clone(), wallet.clone()), locks);
}

pub fn get_total_locked(e: &Env, token: &Address, wallet: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&InitialLockupStorageKey::TotalLocked(token.clone(), wallet.clone()))
        .unwrap_or_default()
}

pub fn set_total_locked(e: &Env, token: &Address, wallet: &Address, amount: i128) {
    e.storage()
        .persistent()
        .set(&InitialLockupStorageKey::TotalLocked(token.clone(), wallet.clone()), &amount);
}

pub fn get_internal_balance(e: &Env, token: &Address, wallet: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&InitialLockupStorageKey::InternalBalance(token.clone(), wallet.clone()))
        .unwrap_or_default()
}

pub fn set_internal_balance(e: &Env, token: &Address, wallet: &Address, balance: i128) {
    e.storage()
        .persistent()
        .set(&InitialLockupStorageKey::InternalBalance(token.clone(), wallet.clone()), &balance);
}
