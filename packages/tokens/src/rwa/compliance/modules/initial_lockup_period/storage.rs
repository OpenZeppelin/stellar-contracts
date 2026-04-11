use soroban_sdk::{contracttype, vec, Address, Env, Vec};

use super::LockupPeriodSet;
use crate::rwa::compliance::{
    modules::{
        storage::{
            add_i128_or_panic, hooks_verified, require_non_negative_amount, sub_i128_or_panic,
            verify_required_hooks,
        },
        MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    ComplianceHook,
};

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

// ################## RAW STORAGE ##################

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

// ################## HELPERS ##################

fn calculate_unlocked_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let now = e.ledger().timestamp();
    let mut unlocked = 0i128;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if lock.release_timestamp <= now {
            unlocked = add_i128_or_panic(e, unlocked, lock.amount);
        }
    }
    unlocked
}

fn calculate_total_locked_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let mut total = 0i128;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        require_non_negative_amount(e, lock.amount);
        total = add_i128_or_panic(e, total, lock.amount);
    }
    total
}

fn update_locked_tokens(e: &Env, token: &Address, wallet: &Address, mut amount_to_consume: i128) {
    let locks = get_locks(e, token, wallet);
    let now = e.ledger().timestamp();
    let mut new_locks = Vec::new(e);
    let mut consumed_total = 0i128;

    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if amount_to_consume > 0 && lock.release_timestamp <= now {
            if amount_to_consume >= lock.amount {
                amount_to_consume = sub_i128_or_panic(e, amount_to_consume, lock.amount);
                consumed_total = add_i128_or_panic(e, consumed_total, lock.amount);
            } else {
                consumed_total = add_i128_or_panic(e, consumed_total, amount_to_consume);
                new_locks.push_back(LockedTokens {
                    amount: sub_i128_or_panic(e, lock.amount, amount_to_consume),
                    release_timestamp: lock.release_timestamp,
                });
                amount_to_consume = 0;
            }
        } else {
            new_locks.push_back(lock);
        }
    }

    set_locks(e, token, wallet, &new_locks);

    let total_locked = get_total_locked(e, token, wallet);
    set_total_locked(e, token, wallet, sub_i128_or_panic(e, total_locked, consumed_total));
}

// ################## ACTIONS ##################

/// Configures the lockup period for `token` and emits [`LockupPeriodSet`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `lockup_seconds` - The lockup duration in seconds.
pub fn configure_lockup_period(e: &Env, token: &Address, lockup_seconds: u64) {
    set_lockup_period(e, token, lockup_seconds);
    LockupPeriodSet { token: token.clone(), lockup_seconds }.publish(e);
}

/// Pre-seeds the lockup state for a wallet. Validates that total locked
/// does not exceed balance.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `balance` - The wallet balance.
/// * `locks` - The lock entries.
pub fn pre_set_lockup_state(
    e: &Env,
    token: &Address,
    wallet: &Address,
    balance: i128,
    locks: &Vec<LockedTokens>,
) {
    require_non_negative_amount(e, balance);

    let total_locked = calculate_total_locked_amount(e, locks);
    assert!(
        total_locked <= balance,
        "InitialLockupPeriodModule: total locked amount cannot exceed balance"
    );

    set_internal_balance(e, token, wallet, balance);
    set_locks(e, token, wallet, locks);
    set_total_locked(e, token, wallet, total_locked);
}

// ################## HOOK WIRING ##################

/// Returns the set of compliance hooks this module requires.
pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
    vec![
        e,
        ComplianceHook::CanTransfer,
        ComplianceHook::Created,
        ComplianceHook::Transferred,
        ComplianceHook::Destroyed,
    ]
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on all required hooks.
pub fn verify_hook_wiring(e: &Env) {
    verify_required_hooks(e, required_hooks(e));
}

// ################## COMPLIANCE HOOKS ##################

/// Updates internal balances and lock tracking after a transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `to` - The recipient address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
pub fn on_transfer(e: &Env, from: &Address, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let total_locked = get_total_locked(e, token, from);

    if total_locked > 0 {
        let pre_balance = get_internal_balance(e, token, from);
        let pre_free = pre_balance - total_locked;

        if amount > pre_free.max(0) {
            let to_consume = amount - pre_free.max(0);
            update_locked_tokens(e, token, from, to_consume);
        }
    }

    let from_bal = get_internal_balance(e, token, from);
    set_internal_balance(e, token, from, sub_i128_or_panic(e, from_bal, amount));

    let to_bal = get_internal_balance(e, token, to);
    set_internal_balance(e, token, to, add_i128_or_panic(e, to_bal, amount));
}

/// Updates internal balance and creates a lock entry after a mint.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient address.
/// * `amount` - The minted amount.
/// * `token` - The token address.
pub fn on_created(e: &Env, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let period = get_lockup_period(e, token);
    if period > 0 {
        let mut locks = get_locks(e, token, to);
        locks.push_back(LockedTokens {
            amount,
            release_timestamp: e.ledger().timestamp().saturating_add(period),
        });
        set_locks(e, token, to, &locks);

        let total = get_total_locked(e, token, to);
        set_total_locked(e, token, to, add_i128_or_panic(e, total, amount));
    }

    let current = get_internal_balance(e, token, to);
    set_internal_balance(e, token, to, add_i128_or_panic(e, current, amount));
}

/// Updates internal balance and consumes locks after a burn.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The burner address.
/// * `amount` - The burned amount.
/// * `token` - The token address.
pub fn on_destroyed(e: &Env, from: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let total_locked = get_total_locked(e, token, from);

    if total_locked > 0 {
        let pre_balance = get_internal_balance(e, token, from);
        let mut free_amount = pre_balance - total_locked;

        if free_amount < amount {
            let locks = get_locks(e, token, from);
            free_amount += calculate_unlocked_amount(e, &locks);
        }

        assert!(
            free_amount >= amount,
            "InitialLockupPeriodModule: insufficient unlocked balance for burn"
        );

        let pre_free = pre_balance - total_locked;
        if amount > pre_free.max(0) {
            let to_consume = amount - pre_free.max(0);
            update_locked_tokens(e, token, from, to_consume);
        }
    }

    let current = get_internal_balance(e, token, from);
    set_internal_balance(e, token, from, sub_i128_or_panic(e, current, amount));
}

/// Returns `true` if the sender has sufficient unlocked balance for the
/// transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
pub fn can_transfer(e: &Env, from: &Address, amount: i128, token: &Address) -> bool {
    assert!(
        hooks_verified(e),
        "InitialLockupPeriodModule: not armed — call verify_hook_wiring() after wiring hooks \
         [CanTransfer, Created, Transferred, Destroyed]"
    );
    if amount < 0 {
        return false;
    }

    let total_locked = get_total_locked(e, token, from);
    if total_locked == 0 {
        return true;
    }

    let balance = get_internal_balance(e, token, from);
    let free = balance - total_locked;

    if free >= amount {
        return true;
    }

    let locks = get_locks(e, token, from);
    let unlocked = calculate_unlocked_amount(e, &locks);
    (free + unlocked) >= amount
}
