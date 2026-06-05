use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::compliance::modules::{
    initial_lockup_period::{
        emit_lockup_period_set, emit_lockup_state_preset, emit_preset_completed,
    },
    storage::{add_i128_or_panic, require_non_negative_amount, sub_i128_or_panic},
    ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

/// A single mint-created lock: `amount` tokens that release at
/// `release_ledger` (ledger timestamp, in seconds).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedTokens {
    pub amount: i128,
    pub release_ledger: u64,
}

/// The lock entries tracked for one `(token, wallet)` pair, together with
/// their running aggregate. `total_locked` always equals the sum of the
/// `locks` amounts, including entries whose release time has already
/// passed: expired entries are consumed lazily by transfers and burns, not
/// by the passage of time.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedDetails {
    pub total_locked: i128,
    pub locks: Vec<LockedTokens>,
}

#[contracttype]
#[derive(Clone)]
pub enum InitialLockupPeriodStorageKey {
    /// Per-token lockup duration in ledgers applied to minted tokens.
    LockupPeriod(Address),
    /// Per-(token, wallet) lock entries and their aggregate.
    LockedDetails(Address, Address),
    /// Per-(token, wallet) balance mirror maintained by the hooks.
    Balance(Address, Address),
    /// Per-token flag indicating that the preset migration phase is
    /// finalized.
    PresetCompleted(Address),
}

// ################## QUERY STATE ##################

/// Returns the configured lockup period for `token`, in ledgers. Returns
/// `0` when no period has been configured, in which case mints do not
/// create locks.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_lockup_period(e: &Env, token: &Address) -> u32 {
    let key = InitialLockupPeriodStorageKey::LockupPeriod(token.clone());
    if let Some(value) = e.storage().persistent().get::<_, u32>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        0
    }
}

/// Returns the lock entries and their aggregate tracked for `wallet` under
/// `token`. Returns an empty [`LockedDetails`] when no entry exists.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
pub fn get_locked_details(e: &Env, token: &Address, wallet: &Address) -> LockedDetails {
    let key = InitialLockupPeriodStorageKey::LockedDetails(token.clone(), wallet.clone());
    if let Some(value) = e.storage().persistent().get::<_, LockedDetails>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        LockedDetails { total_locked: 0, locks: Vec::new(e) }
    }
}

/// Returns the balance mirror tracked for `wallet` under `token`. Returns
/// `0` when no entry exists.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
pub fn get_tracked_balance(e: &Env, token: &Address, wallet: &Address) -> i128 {
    let key = InitialLockupPeriodStorageKey::Balance(token.clone(), wallet.clone());
    if let Some(value) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        0
    }
}

/// Returns the amount `wallet` can currently spend under `token`: the
/// tracked balance minus lock entries that have not released yet.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathOverflow`] - When summing the expired lock
///   amounts overflows.
pub fn get_unlocked_balance(e: &Env, token: &Address, wallet: &Address) -> i128 {
    let details = get_locked_details(e, token, wallet);
    unlocked_balance(e, token, wallet, &details)
}

/// Returns `true` when the preset phase for `token` has been finalized,
/// blocking any further preset writes.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn is_preset_completed(e: &Env, token: &Address) -> bool {
    let key = InitialLockupPeriodStorageKey::PresetCompleted(token.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Returns `true` when the sender's unlocked holdings cover `amount`.
///
/// Only the sender side is consulted: the recipient is intentionally
/// ignored because incoming transfers never create locks.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `_to` - The recipient address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MathOverflow`] - When summing the expired lock
///   amounts overflows.
pub fn can_transfer(e: &Env, from: &Address, _to: &Address, amount: i128, token: &Address) -> bool {
    require_non_negative_amount(e, amount);
    let details = get_locked_details(e, token, from);
    amount <= unlocked_balance(e, token, from, &details)
}

// ################## CHANGE STATE ##################

/// Sets the lockup period for `token`, in ledgers. Affects only locks
/// created by subsequent mints; existing lock entries keep their original
/// release times.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `period` - The lockup duration in ledgers. `0` disables locking for future
///   mints.
///
/// # Events
///
/// * topics - `["lockup_period_set", token: Address]`
/// * data - `[period: u32]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_lockup_period(e: &Env, token: &Address, period: u32) {
    let key = InitialLockupPeriodStorageKey::LockupPeriod(token.clone());
    e.storage().persistent().set(&key, &period);
    emit_lockup_period_set(e, token, period);
}

/// Pre-seeds the lockup state for `wallet` under `token`: the balance
/// mirror and any pre-existing lock entries. Useful when registering this
/// module on a token that already has live balances.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `balance` - The wallet's token balance to mirror.
/// * `locks` - The lock entries to record for `wallet`.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `balance` or any lock
///   amount is negative.
/// * [`ComplianceModuleError::PresetAlreadyCompleted`] - When the preset phase
///   has already been finalized.
/// * [`ComplianceModuleError::MathOverflow`] - When summing the lock amounts
///   overflows.
/// * [`ComplianceModuleError::LockedAmountExceedsBalance`] - When the aggregate
///   locked amount exceeds `balance`.
///
/// # Events
///
/// * topics - `["lockup_state_preset", token: Address, wallet: Address]`
/// * data - `[balance: i128, total_locked: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn preset_lockup_state(
    e: &Env,
    token: &Address,
    wallet: &Address,
    balance: i128,
    locks: &Vec<LockedTokens>,
) {
    require_non_negative_amount(e, balance);
    if is_preset_completed(e, token) {
        panic_with_error!(e, ComplianceModuleError::PresetAlreadyCompleted);
    }

    let mut total_locked = 0;
    for lock in locks.iter() {
        require_non_negative_amount(e, lock.amount);
        total_locked = add_i128_or_panic(e, total_locked, lock.amount);
    }
    if total_locked > balance {
        panic_with_error!(e, ComplianceModuleError::LockedAmountExceedsBalance);
    }

    set_tracked_balance(e, token, wallet, balance);
    set_locked_details(e, token, wallet, &LockedDetails { total_locked, locks: locks.clone() });
    emit_lockup_state_preset(e, token, wallet, balance, total_locked);
}

/// Finalizes the preset phase for `token`. After this call, invoking
/// [`preset_lockup_state`] will panic with
/// [`ComplianceModuleError::PresetAlreadyCompleted`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
///
/// # Events
///
/// * topics - `["preset_completed", token: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn mark_preset_completed(e: &Env, token: &Address) {
    let key = InitialLockupPeriodStorageKey::PresetCompleted(token.clone());
    e.storage().persistent().set(&key, &());
    emit_preset_completed(e, token);
}

/// Records a transfer between two wallets: debits the sender (consuming
/// expired locks when the amount dips into the locked region) and credits
/// the recipient's balance mirror. Panics when the sender's unlocked
/// holdings do not cover `amount`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender wallet.
/// * `to` - The recipient wallet.
/// * `amount` - The transferred amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::InsufficientUnlockedBalance`] - When `amount`
///   exceeds the sender's unlocked holdings.
/// * [`ComplianceModuleError::MathUnderflow`] - When the sender's balance
///   mirror would go negative.
/// * [`ComplianceModuleError::MathOverflow`] - When the recipient's credit
///   addition overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_transfer(e: &Env, from: &Address, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    debit_unlocked(e, token, from, amount);
    credit_balance(e, token, to, amount);
}

/// Records a mint to `to`: credits the recipient's balance mirror and, when
/// a lockup period is configured, appends a lock entry releasing after that
/// period.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient wallet.
/// * `amount` - The minted amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MathOverflow`] - When the lock aggregate or the
///   credit addition overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_created(e: &Env, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let period = get_lockup_period(e, token);
    if period > 0 && amount > 0 {
        let mut details = get_locked_details(e, token, to);
        details.locks.push_back(LockedTokens {
            amount,
            release_ledger: e.ledger().sequence().saturating_add(period),
        });
        details.total_locked = add_i128_or_panic(e, details.total_locked, amount);
        set_locked_details(e, token, to, &details);
    }

    credit_balance(e, token, to, amount);
}

/// Records a burn from `from`: debits the wallet, consuming expired locks
/// when the amount dips into the locked region. Panics when the wallet's
/// unlocked holdings do not cover `amount`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The wallet whose tokens were burned.
/// * `amount` - The burned amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::InsufficientUnlockedBalance`] - When `amount`
///   exceeds the wallet's unlocked holdings.
/// * [`ComplianceModuleError::MathUnderflow`] - When the wallet's balance
///   mirror would go negative.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_destroyed(e: &Env, from: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    debit_unlocked(e, token, from, amount);
}

// ################## LOW-LEVEL HELPERS ##################

/// Writes the lock entries for `(token, wallet)` directly to persistent
/// storage, replacing any existing value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `details` - The lock entries and aggregate to record.
///
/// # Security Warning
///
/// This helper performs no authorization checks and does not validate that
/// `total_locked` matches the lock entries or stays within the balance
/// mirror. Callers must enforce both invariants themselves.
pub fn set_locked_details(e: &Env, token: &Address, wallet: &Address, details: &LockedDetails) {
    let key = InitialLockupPeriodStorageKey::LockedDetails(token.clone(), wallet.clone());
    e.storage().persistent().set(&key, details);
}

/// Writes the balance mirror for `(token, wallet)` directly to persistent
/// storage, replacing any existing value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `balance` - The new balance to record.
///
/// # Security Warning
///
/// This helper performs no authorization checks and does not validate the
/// balance against the wallet's locked amounts. Callers must keep the
/// mirror consistent themselves.
pub fn set_tracked_balance(e: &Env, token: &Address, wallet: &Address, balance: i128) {
    let key = InitialLockupPeriodStorageKey::Balance(token.clone(), wallet.clone());
    e.storage().persistent().set(&key, &balance);
}

/// Credits `amount` to `wallet`'s balance mirror under `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `amount` - The amount to credit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathOverflow`] - When adding `amount` to the
///   current balance overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn credit_balance(e: &Env, token: &Address, wallet: &Address, amount: i128) {
    let balance = get_tracked_balance(e, token, wallet);
    set_tracked_balance(e, token, wallet, add_i128_or_panic(e, balance, amount));
}

/// Debits `amount` from `wallet`'s holdings under `token`: when the amount
/// dips into the locked region, expired lock entries are consumed
/// oldest-first before the balance mirror is decremented.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `amount` - The amount to debit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::InsufficientUnlockedBalance`] - When `amount`
///   exceeds the wallet's unlocked holdings.
/// * [`ComplianceModuleError::MathUnderflow`] - When the balance mirror would
///   go negative.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn debit_unlocked(e: &Env, token: &Address, wallet: &Address, amount: i128) {
    let balance = get_tracked_balance(e, token, wallet);
    let mut details = get_locked_details(e, token, wallet);

    if details.total_locked > 0 {
        let free = (balance - details.total_locked).max(0);
        if amount > free {
            consume_expired_locks(e, &mut details, sub_i128_or_panic(e, amount, free));
            set_locked_details(e, token, wallet, &details);
        }
    }

    let next = sub_i128_or_panic(e, balance, amount);
    if next < 0 {
        panic_with_error!(e, ComplianceModuleError::MathUnderflow);
    }
    set_tracked_balance(e, token, wallet, next);
}

/// Returns the amount `wallet` can currently spend, given its already-read
/// `details`: the free portion of the balance mirror plus expired locks.
fn unlocked_balance(e: &Env, token: &Address, wallet: &Address, details: &LockedDetails) -> i128 {
    let balance = get_tracked_balance(e, token, wallet);
    let free = (balance - details.total_locked).max(0);
    add_i128_or_panic(e, free, expired_amount(e, &details.locks))
}

/// Returns the sum of lock amounts whose release time has passed.
fn expired_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let now = e.ledger().sequence();
    let mut expired = 0;
    for lock in locks.iter() {
        if lock.release_ledger <= now {
            expired = add_i128_or_panic(e, expired, lock.amount);
        }
    }
    expired
}

/// Consumes `amount` from the expired entries in `details`, oldest-first:
/// fully-consumed entries are dropped, a partially-consumed entry keeps its
/// remainder, and `total_locked` is reduced by `amount`. Panics when the
/// expired entries cannot cover `amount`.
fn consume_expired_locks(e: &Env, details: &mut LockedDetails, amount: i128) {
    let now = e.ledger().sequence();
    let mut to_consume = amount;
    let mut remaining = Vec::new(e);

    for lock in details.locks.iter() {
        if to_consume > 0 && lock.release_ledger <= now {
            if lock.amount <= to_consume {
                to_consume -= lock.amount;
            } else {
                remaining.push_back(LockedTokens {
                    amount: lock.amount - to_consume,
                    release_ledger: lock.release_ledger,
                });
                to_consume = 0;
            }
        } else {
            remaining.push_back(lock);
        }
    }

    if to_consume > 0 {
        panic_with_error!(e, ComplianceModuleError::InsufficientUnlockedBalance);
    }

    details.total_locked = sub_i128_or_panic(e, details.total_locked, amount);
    details.locks = remaining;
}
