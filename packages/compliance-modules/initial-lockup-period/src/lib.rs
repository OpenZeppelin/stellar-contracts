#![no_std]

//! Initial lockup period compliance module — Stellar port of T-REX
//! [`InitialLockupPeriodModule.sol`][trex-src].
//!
//! Enforces a lockup period for all investors whenever they receive tokens
//! through primary emissions (mints). Tokens received via peer-to-peer
//! transfers are **not** subject to lockup restrictions.
//!
//! Each mint creates a separate lock entry with its own release timestamp,
//! enabling partial unlocking: if an investor receives two mints at different
//! times, each unlocks independently.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                       |
//! |------------------------|-----------------|-------------------------------------------------|
//! | `moduleMintAction`     | `on_created`    | Push lock entry, increase `total_locked`        |
//! | `moduleTransferAction` | `on_transfer`   | Consume expired entries when locked tokens move |
//! | `moduleBurnAction`     | `on_destroyed`  | Prevent burning still-locked tokens             |
//! | `moduleCheck`          | `can_transfer`  | Allow transfer only if free balance >= amount   |
//!
//! ## Differences from T-REX
//!
//! - Lockup period is configured in **seconds** (Soroban ledger timestamps)
//!   rather than days (T-REX multiplies days × 86 400 internally).
//! - `update_locked_tokens` also decrements `total_locked` to keep the
//!   counter accurate. The T-REX version leaves `total_locked` stale and
//!   compensates at read time via `_calculateUnlockedAmount`.
//! - Uses `i128` (Soroban native) instead of `uint256`, naturally avoiding
//!   underflow when `total_locked` exceeds post-transfer balance.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/4.2.0-beta2/contracts/compliance/modular/modules/InitialLockupPeriodModule.sol

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env, Vec};

use stellar_tokens::rwa::compliance::ComplianceModule;

use stellar_compliance_common::{
    checked_add_i128, get_compliance_address, module_name, require_compliance_auth,
    require_non_negative_amount, set_compliance_address, TokenBalanceViewClient,
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
enum DataKey {
    /// Per-token lockup duration in seconds.
    LockupPeriod(Address),
    /// Per-(token, wallet) ordered list of individual lock entries.
    Locks(Address, Address),
    /// Per-(token, wallet) aggregate of all locked amounts.
    /// Equivalent to T-REX `LockedDetails.totalLocked`.
    TotalLocked(Address, Address),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub lockup_seconds: u64,
}

#[contract]
pub struct InitialLockupPeriodModule;

// ---------------------------------------------------------------------------
// Admin / query API
// ---------------------------------------------------------------------------

#[contractimpl]
impl InitialLockupPeriodModule {
    /// Configures the lockup duration for newly minted tokens.
    /// T-REX equivalent: `setLockupPeriod(_lockupPeriodInDays)`.
    pub fn set_lockup_period(e: &Env, token: Address, lockup_seconds: u64) {
        require_compliance_auth(e);
        e.storage()
            .persistent()
            .set(&DataKey::LockupPeriod(token.clone()), &lockup_seconds);
        LockupPeriodSet { token, lockup_seconds }.publish(e);
    }

    pub fn get_lockup_period(e: &Env, token: Address) -> u64 {
        e.storage()
            .persistent()
            .get(&DataKey::LockupPeriod(token))
            .unwrap_or_default()
    }

    pub fn get_total_locked(e: &Env, token: Address, wallet: Address) -> i128 {
        e.storage()
            .persistent()
            .get(&DataKey::TotalLocked(token, wallet))
            .unwrap_or_default()
    }

    pub fn get_locked_tokens(e: &Env, token: Address, wallet: Address) -> Vec<LockedTokens> {
        e.storage()
            .persistent()
            .get(&DataKey::Locks(token, wallet))
            .unwrap_or_else(|| Vec::new(e))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers (mirror T-REX _calculateUnlockedAmount / _updateLockedTokens)
// ---------------------------------------------------------------------------

/// Sum of amounts from expired lock entries (`release_timestamp <= now`).
/// Pure read — no state mutation. Mirrors T-REX `_calculateUnlockedAmount`.
fn calculate_unlocked_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let now = e.ledger().timestamp();
    let mut unlocked = 0i128;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if lock.release_timestamp <= now {
            unlocked += lock.amount;
        }
    }
    unlocked
}

/// Consumes `amount_to_consume` from expired lock entries (positional order),
/// removes fully consumed entries, and decrements `total_locked` by the
/// amount actually consumed.
///
/// Mirrors T-REX `_updateLockedTokens` but also maintains `total_locked`
/// consistency — the Solidity version does not, relying on read-time
/// compensation via `_calculateUnlockedAmount`.
fn update_locked_tokens(e: &Env, token: &Address, wallet: &Address, mut amount_to_consume: i128) {
    let locks_key = DataKey::Locks(token.clone(), wallet.clone());
    let locks: Vec<LockedTokens> = e
        .storage()
        .persistent()
        .get(&locks_key)
        .unwrap_or_else(|| Vec::new(e));

    let now = e.ledger().timestamp();
    let mut new_locks = Vec::new(e);
    let mut consumed_total = 0i128;

    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if amount_to_consume > 0 && lock.release_timestamp <= now {
            if amount_to_consume >= lock.amount {
                amount_to_consume -= lock.amount;
                consumed_total += lock.amount;
            } else {
                consumed_total += amount_to_consume;
                new_locks.push_back(LockedTokens {
                    amount: lock.amount - amount_to_consume,
                    release_timestamp: lock.release_timestamp,
                });
                amount_to_consume = 0;
            }
        } else {
            new_locks.push_back(lock);
        }
    }

    e.storage().persistent().set(&locks_key, &new_locks);

    let total_key = DataKey::TotalLocked(token.clone(), wallet.clone());
    let total_locked: i128 = e
        .storage()
        .persistent()
        .get(&total_key)
        .unwrap_or_default();
    e.storage()
        .persistent()
        .set(&total_key, &(total_locked - consumed_total));
}

// ---------------------------------------------------------------------------
// ComplianceModule trait implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl ComplianceModule for InitialLockupPeriodModule {
    /// T-REX `moduleTransferAction`: after a P2P transfer, consume expired
    /// lock entries if the transfer ate into the "locked" portion of balance.
    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let total_locked = Self::get_total_locked(e, token.clone(), from.clone());
        if total_locked == 0 {
            return;
        }

        // Balance is post-transfer; reconstruct pre-transfer balance.
        let post_balance = TokenBalanceViewClient::new(e, &token).balance(&from);
        let pre_balance = checked_add_i128(e, post_balance, amount);
        let pre_free = pre_balance - total_locked;

        if amount > pre_free.max(0) {
            let to_consume = amount - pre_free.max(0);
            update_locked_tokens(e, &token, &from, to_consume);
        }
    }

    /// T-REX `moduleMintAction`: push a new lock entry for the minted amount.
    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let period = Self::get_lockup_period(e, token.clone());
        if period == 0 {
            return;
        }

        let locks_key = DataKey::Locks(token.clone(), to.clone());
        let mut locks: Vec<LockedTokens> = e
            .storage()
            .persistent()
            .get(&locks_key)
            .unwrap_or_else(|| Vec::new(e));

        locks.push_back(LockedTokens {
            amount,
            release_timestamp: e.ledger().timestamp().saturating_add(period),
        });
        e.storage().persistent().set(&locks_key, &locks);

        let total_key = DataKey::TotalLocked(token, to);
        let total: i128 = e
            .storage()
            .persistent()
            .get(&total_key)
            .unwrap_or_default();
        e.storage()
            .persistent()
            .set(&total_key, &checked_add_i128(e, total, amount));
    }

    /// T-REX `moduleBurnAction`: panics if the burn would consume
    /// still-locked (non-expired) tokens, otherwise cleans up expired entries.
    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let total_locked = Self::get_total_locked(e, token.clone(), from.clone());
        if total_locked == 0 {
            return;
        }

        // Balance is post-burn; reconstruct pre-burn balance.
        let post_balance = TokenBalanceViewClient::new(e, &token).balance(&from);
        let pre_balance = checked_add_i128(e, post_balance, amount);
        let mut free_amount = pre_balance - total_locked;

        if free_amount < amount {
            let locks = Self::get_locked_tokens(e, token.clone(), from.clone());
            free_amount += calculate_unlocked_amount(e, &locks);
        }

        assert!(
            free_amount >= amount,
            "InitialLockupPeriodModule: insufficient unlocked balance for burn"
        );

        // Clean up expired entries if the burn consumed some.
        let pre_free = pre_balance - total_locked;
        if amount > pre_free.max(0) {
            let to_consume = amount - pre_free.max(0);
            update_locked_tokens(e, &token, &from, to_consume);
        }
    }

    /// T-REX `moduleCheck`: allow transfer only if free balance >= amount.
    /// Free balance = `balance - totalLocked + sum(expired_entries)`.
    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        if amount < 0 {
            return false;
        }

        let total_locked = Self::get_total_locked(e, token.clone(), from.clone());
        if total_locked == 0 {
            return true;
        }

        let balance = TokenBalanceViewClient::new(e, &token).balance(&from);
        let free = balance - total_locked;

        // Fast path: enough free tokens without inspecting individual entries.
        if free >= amount {
            return true;
        }

        // Accurate path: account for expired-but-not-yet-cleaned-up entries.
        let locks = Self::get_locked_tokens(e, token, from);
        let unlocked = calculate_unlocked_amount(e, &locks);
        (free + unlocked) >= amount
    }

    /// Minting is always allowed — it creates the lock entries, not blocks them.
    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "InitialLockupPeriodModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
