use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::compliance::{
    modules::{
        initial_lockup_period::{
            emit_lockup_period_set, emit_lockup_state_preset, emit_preset_completed,
        },
        storage::{add_i128_or_panic, require_non_negative_amount, sub_i128_or_panic},
        ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    TransferKind,
};

/// A single mint-created lock: `amount` tokens that release once the
/// ledger sequence reaches `release_ledger`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedTokens {
    pub amount: i128,
    pub release_ledger: u32,
}

/// The lock entries tracked for one `(token, wallet)` pair, together with
/// their running aggregate. `total_locked` always equals the sum of the
/// `locks` amounts, including entries whose release time has already
/// passed: expired entries are consumed lazily by transfers and burns and
/// pruned by subsequent mints, not by the passage of time.
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

/// Returns the amount still locked for `wallet` under `token`: the aggregate
/// of lock entries whose release time has not yet passed. Returns `0` when no
/// entry exists.
///
/// This module tracks locks, not balances. A caller that wants the spendable
/// (unlocked) amount subtracts this from the wallet's token balance:
/// `unlocked = balance - locked_amount`.
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
pub fn get_locked_amount(e: &Env, token: &Address, wallet: &Address) -> i128 {
    let details = get_locked_details(e, token, wallet);
    sub_i128_or_panic(e, details.total_locked, expired_amount(e, &details.locks))
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

/// Pre-seeds the lock entries for `wallet` under `token`. Useful when
/// registering this module on a token whose holders must start with locks
/// already in place.
///
/// Unlike the upstream Solidity module, no balance is seeded: each wallet's
/// balance is read from the token on every hook, so only the lock schedule
/// needs migrating. Holders with no pre-existing locks need no preset at all.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `locks` - The lock entries to record.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When any lock amount is
///   negative.
/// * [`ComplianceModuleError::PresetAlreadyCompleted`] - When the preset phase
///   has already been finalized.
/// * [`ComplianceModuleError::MathOverflow`] - When summing the lock amounts
///   overflows.
///
/// # Events
///
/// * topics - `["lockup_state_preset", token: Address, wallet: Address]`
/// * data - `[total_locked: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn preset_locks(e: &Env, token: &Address, wallet: &Address, locks: &Vec<LockedTokens>) {
    if is_preset_completed(e, token) {
        panic_with_error!(e, ComplianceModuleError::PresetAlreadyCompleted);
    }

    let mut total_locked = 0;
    for lock in locks.iter() {
        require_non_negative_amount(e, lock.amount);
        total_locked = add_i128_or_panic(e, total_locked, lock.amount);
    }

    set_locked_details(e, token, wallet, &LockedDetails { total_locked, locks: locks.clone() });
    emit_lockup_state_preset(e, token, wallet, total_locked);
}

/// Finalizes the preset phase for `token`. After this call, invoking
/// [`preset_locks`] will panic with
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

/// Records a transfer out of `from`, dispatching on `kind`:
///
/// * Standard and delegated transfers consume expired locks when `amount` dips
///   into the locked region, and panic when `from`'s unlocked holdings do not
///   cover `amount`. The recipient is untouched, since incoming transfers never
///   create locks.
/// * Forced transfers (seizures) override the lockup policy instead of being
///   rejected: the moved amount is covered by consuming locks oldest-first,
///   expired or not. The consumed entries leave the books with the seized
///   tokens; the recipient gains nothing.
/// * Recovery transfers migrate instead of consuming: the lock entries covering
///   the moved amount leave `from` and are re-created on `to` with their
///   release times preserved, so the investor's remaining lockup follows the
///   balance onto the new wallet.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender wallet.
/// * `to` - The recipient wallet.
/// * `balance` - The sender's token balance, as of before the transfer.
/// * `amount` - The transferred amount.
/// * `kind` - Who initiated the transfer and under what authority.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::InsufficientUnlockedBalance`] - When `amount`
///   exceeds the sender's unlocked holdings and the transfer is not privileged.
/// * [`ComplianceModuleError::MathOverflow`] - When migrating locks onto the
///   recipient overflows its lock aggregate.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_transfer(
    e: &Env,
    from: &Address,
    to: &Address,
    balance: i128,
    amount: i128,
    kind: &TransferKind,
    token: &Address,
) {
    require_non_negative_amount(e, amount);
    match kind {
        TransferKind::Forced => debit_forced(e, token, from, balance, amount),
        TransferKind::Recovery => migrate_locks(e, token, from, to, balance, amount),
        TransferKind::Standard | TransferKind::Delegated(_) => {
            debit_unlocked(e, token, from, balance, amount)
        }
    }
}

/// Records a mint to `to`: when a lockup period is configured, appends a lock
/// entry releasing after that period. Expired entries are pruned in the same
/// write, so the stored vector tracks the wallet's active locks rather than
/// its lifetime mint count and cannot grow past the ledger-entry size limit
/// under repeated issuance. The wallet's balance is owned by the token, so
/// nothing else is tracked.
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
/// * [`ComplianceModuleError::MathOverflow`] - When the lock aggregate
///   overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_created(e: &Env, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let period = get_lockup_period(e, token);
    if period > 0 && amount > 0 {
        let mut details = get_locked_details(e, token, to);
        prune_expired_locks(e, &mut details);
        details.locks.push_back(LockedTokens {
            amount,
            release_ledger: e.ledger().sequence().saturating_add(period),
        });
        details.total_locked = add_i128_or_panic(e, details.total_locked, amount);
        set_locked_details(e, token, to, &details);
    }
}

/// Records a burn from `from`: consumes expired locks when `amount` dips into
/// the locked region. Panics when `from`'s unlocked holdings do not cover
/// `amount`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The wallet whose tokens were burned.
/// * `balance` - The wallet's token balance, as of before the burn.
/// * `amount` - The burned amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::InsufficientUnlockedBalance`] - When `amount`
///   exceeds the wallet's unlocked holdings.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_destroyed(e: &Env, from: &Address, balance: i128, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    debit_unlocked(e, token, from, balance, amount);
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
/// `total_locked` matches the lock entries. Callers must enforce the invariant
/// themselves.
pub fn set_locked_details(e: &Env, token: &Address, wallet: &Address, details: &LockedDetails) {
    let key = InitialLockupPeriodStorageKey::LockedDetails(token.clone(), wallet.clone());
    e.storage().persistent().set(&key, details);
}

/// Debits `amount` from `wallet`'s holdings under `token`, given its token
/// `balance` as of before the operation: when the amount dips into the locked
/// region, expired lock entries are consumed oldest-first. The wallet's
/// balance is owned by the token, so nothing is written unless locks are
/// consumed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `balance` - The wallet's token balance, as of before the operation.
/// * `amount` - The amount to debit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::InsufficientUnlockedBalance`] - When `amount`
///   exceeds the wallet's unlocked holdings.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn debit_unlocked(e: &Env, token: &Address, wallet: &Address, balance: i128, amount: i128) {
    let mut details = get_locked_details(e, token, wallet);

    // When the spend reaches past the never-locked portion, the difference
    // must be covered by expired locks: consuming them oldest-first panics
    // with `InsufficientUnlockedBalance` on any shortfall. This single check
    // also rejects spending past the balance itself, since the consumption
    // need then exceeds every lock on the books.
    let free = (balance - details.total_locked).max(0);
    if amount > free {
        consume_expired_locks(e, &mut details, sub_i128_or_panic(e, amount, free));
        set_locked_details(e, token, wallet, &details);
    }
}

/// Debits `amount` from `wallet`'s holdings under `token` for a forced
/// transfer (seizure): when the amount dips into the locked region, lock
/// entries are consumed oldest-first regardless of whether they have
/// released. The lockup policy is an investor-facing rule the admin is
/// consciously overriding, but the lock schedule must still shrink with
/// the departing tokens or it would exceed the wallet's remaining balance.
/// For recovery transfers, which migrate the consumed entries to the new
/// wallet instead of dropping them, see [`migrate_locks`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `wallet` - The wallet address.
/// * `balance` - The wallet's token balance, as of before the operation.
/// * `amount` - The amount to debit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Security Warning
///
/// This helper performs no authorization checks and bypasses the unlocked
/// balance check.
pub fn debit_forced(e: &Env, token: &Address, wallet: &Address, balance: i128, amount: i128) {
    let mut details = get_locked_details(e, token, wallet);

    if details.total_locked > 0 {
        let free = (balance - details.total_locked).max(0);
        if amount > free {
            let _ = consume_locks_forced(e, &mut details, sub_i128_or_panic(e, amount, free));
            set_locked_details(e, token, wallet, &details);
        }
    }
}

/// Migrates the lock entries covering `amount` from `from` to `to` for a
/// recovery transfer: the portion of the moved amount not covered by
/// `from`'s never-locked free balance is consumed from its entries
/// oldest-first, expired or not, and the consumed pieces are re-created on
/// `to` with their release times preserved, merging with any entries `to`
/// already holds. A recovery moves the investor's balance to a new wallet,
/// so the remaining lockup follows the tokens instead of being released
/// early. Consumption stops when the entries are exhausted; it never
/// panics on shortfall.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `from` - The wallet the balance leaves (the lost wallet).
/// * `to` - The wallet the balance arrives on (the investor's new wallet).
/// * `balance` - `from`'s token balance, as of before the operation.
/// * `amount` - The amount to migrate. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathOverflow`] - When crediting `to`'s lock
///   aggregate overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks and bypasses the unlocked
/// balance check.
pub fn migrate_locks(
    e: &Env,
    token: &Address,
    from: &Address,
    to: &Address,
    balance: i128,
    amount: i128,
) {
    let mut src = get_locked_details(e, token, from);
    if src.total_locked == 0 {
        return;
    }

    let free = (balance - src.total_locked).max(0);
    if amount <= free {
        // should be unreachable if called via `recover_balance`, this is just a
        // security belt for the case that the caller is not `recover_balance`
        // and does not check the free balance for example, maybe useful for
        // partial recovery in the future
        return;
    }

    let moved = consume_locks_forced(e, &mut src, sub_i128_or_panic(e, amount, free));
    set_locked_details(e, token, from, &src);

    if moved.is_empty() {
        return;
    }

    // Read the destination only after the source write, so a recovery onto
    // the same wallet observes the already-debited entries and reduces to a
    // no-op instead of duplicating them.
    let mut dst = get_locked_details(e, token, to);
    for lock in moved.iter() {
        dst.total_locked = add_i128_or_panic(e, dst.total_locked, lock.amount);
        dst.locks.push_back(lock);
    }
    set_locked_details(e, token, to, &dst);
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

/// Drops the entries in `details` whose release time has passed, reducing
/// `total_locked` by the dropped amounts. Expired entries are already
/// spendable ([`get_locked_amount`] subtracts them), so dropping them
/// changes nothing about what the wallet can move; it only keeps the
/// stored vector bounded by the number of active locks.
fn prune_expired_locks(e: &Env, details: &mut LockedDetails) {
    let now = e.ledger().sequence();
    let mut pruned = 0;
    let mut remaining = Vec::new(e);

    for lock in details.locks.iter() {
        if lock.release_ledger <= now {
            pruned = add_i128_or_panic(e, pruned, lock.amount);
        } else {
            remaining.push_back(lock);
        }
    }

    if pruned > 0 {
        details.total_locked = sub_i128_or_panic(e, details.total_locked, pruned);
        details.locks = remaining;
    }
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

/// Consumes `amount` from the entries in `details` oldest-first, regardless
/// of release time: fully-consumed entries are dropped, a partially-consumed
/// entry keeps its remainder, and `total_locked` is reduced by what was
/// consumed. Returns the consumed pieces with their release times
/// preserved: [`debit_forced`] discards them (the tokens are seized), while
/// [`migrate_locks`] re-creates them on the destination wallet. Consumption
/// stops when the entries are exhausted; it never panics on shortfall.
fn consume_locks_forced(e: &Env, details: &mut LockedDetails, amount: i128) -> Vec<LockedTokens> {
    let mut to_consume = amount;
    let mut consumed = 0;
    let mut consumed_entries = Vec::new(e);
    let mut remaining = Vec::new(e);

    for lock in details.locks.iter() {
        if to_consume > 0 {
            if lock.amount <= to_consume {
                to_consume -= lock.amount;
                consumed += lock.amount;
                consumed_entries.push_back(lock);
            } else {
                remaining.push_back(LockedTokens {
                    amount: lock.amount - to_consume,
                    release_ledger: lock.release_ledger,
                });
                consumed_entries.push_back(LockedTokens {
                    amount: to_consume,
                    release_ledger: lock.release_ledger,
                });
                consumed += to_consume;
                to_consume = 0;
            }
        } else {
            remaining.push_back(lock);
        }
    }

    details.total_locked = sub_i128_or_panic(e, details.total_locked, consumed);
    details.locks = remaining;
    consumed_entries
}
