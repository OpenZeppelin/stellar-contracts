#![no_std]

//! Initial lockup period compliance module — Stellar port of T-REX
//! [`TimeExchangeLimitsModule.sol`][trex-src].
//!
//! ## Naming
//!
//! The T-REX EVM module is called `TimeExchangeLimitsModule`, but we renamed
//! it to `InitialLockupPeriodModule` for clarity:
//!
//! - The original name suggests time-based *transfer rate limiting* (similar to
//!   `TimeTransfersLimitsModule`), but the actual behaviour is a per-mint
//!   lockup — tokens are frozen for a fixed duration after primary issuance.
//! - Keeping both `TimeExchangeLimitsModule` and `TimeTransfersLimitsModule`
//!   would be confusing since they serve very different purposes.
//!
//! **Open question for reviewers:** should we revert to the original T-REX
//! name (`TimeExchangeLimitsModule`) for stricter 1:1 naming parity?
//!
//! ## Purpose
//!
//! Enforces a lockup period for all investors whenever they receive tokens
//! through primary emissions (mints). Tokens received via peer-to-peer
//! transfers are **not** subject to lockup restrictions.
//!
//! Each mint creates a separate lock entry with its own release timestamp,
//! enabling partial unlocking: if an investor receives two mints at different
//! times, each unlocks independently.
//!
//! ## Internal state tracking
//!
//! Instead of calling `token.balance()` (which would cause a forbidden
//! re-entrant call on Soroban), this module maintains its own internal
//! balance counter per wallet. The counter is updated via the `on_created`,
//! `on_transfer`, and `on_destroyed` hooks, so the module **must** be wired
//! to `Created`, `Transferred`, and `Destroyed` hooks in addition to
//! `CanTransfer`.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                       |
//! |------------------------|-----------------|-------------------------------------------------|
//! | `moduleMintAction`     | `on_created`    | Push lock entry, increase `total_locked`, update internal balance |
//! | `moduleTransferAction` | `on_transfer`   | Consume expired entries, update internal balances |
//! | `moduleBurnAction`     | `on_destroyed`  | Prevent burning still-locked tokens, update internal balance |
//! | `moduleCheck`          | `can_transfer`  | Allow transfer only if free balance >= amount   |
//!
//! ## Required hooks
//!
//! `CanTransfer`, `Created`, `Transferred`, `Destroyed`
//!
//! Call `verify_hook_wiring()` after wiring to arm the module. The
//! `can_transfer` hook panics if the module is not armed — this prevents
//! silent misconfiguration where missing hooks would cause the internal balance
//! counter to drift.
//!
//! ## Differences from T-REX
//!
//! - Lockup period is configured in **seconds** (Soroban ledger timestamps)
//!   rather than days (T-REX multiplies days × 86 400 internally).
//! - `update_locked_tokens` also decrements `total_locked` to keep the counter
//!   accurate. The T-REX version leaves `total_locked` stale and compensates at
//!   read time via `_calculateUnlockedAmount`.
//! - Uses `i128` (Soroban native) instead of `uint256`, naturally avoiding
//!   underflow when `total_locked` exceeds post-transfer balance.
//! - Uses internal balance counter instead of `token.balance()` to avoid
//!   Soroban's contract re-entry restriction.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeExchangeLimitsModule.sol

pub mod storage;

use soroban_sdk::{contract, contractevent, contractimpl, vec, Address, Env, Vec};
use stellar_compliance_common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, hooks_verified, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address,
    verify_required_hooks,
};
use stellar_tokens::rwa::compliance::{ComplianceHook, ComplianceModule};

pub use storage::LockedTokens;
use storage::{
    get_internal_balance, get_lockup_period, get_locks, get_total_locked, set_internal_balance,
    set_lockup_period, set_locks, set_total_locked,
};

/// Emitted when a token's lockup duration is configured or changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub lockup_seconds: u64,
}

/// Enforces a per-mint lockup period after primary issuance.
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
        set_lockup_period(e, &token, lockup_seconds);
        LockupPeriodSet { token, lockup_seconds }.publish(e);
    }

    /// Returns the configured lockup duration (seconds) for `token`.
    pub fn get_lockup_period(e: &Env, token: Address) -> u64 {
        get_lockup_period(e, &token)
    }

    /// Returns the aggregate locked amount for a `(token, wallet)` pair.
    pub fn get_total_locked(e: &Env, token: Address, wallet: Address) -> i128 {
        get_total_locked(e, &token, &wallet)
    }

    /// Returns the ordered list of individual lock entries for a wallet.
    pub fn get_locked_tokens(e: &Env, token: Address, wallet: Address) -> Vec<LockedTokens> {
        get_locks(e, &token, &wallet)
    }

    /// Returns the module's internal balance mirror for a wallet.
    pub fn get_internal_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        get_internal_balance(e, &token, &wallet)
    }

    /// Returns the compliance hooks this module must be registered on.
    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![
            e,
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ]
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
}

// ---------------------------------------------------------------------------
// Internal helpers (mirror T-REX _calculateUnlockedAmount /
// _updateLockedTokens)
// ---------------------------------------------------------------------------

/// Sum of amounts from expired lock entries (`release_timestamp <= now`).
/// Pure read — no state mutation. Mirrors T-REX `_calculateUnlockedAmount`.
fn calculate_unlocked_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let now = e.ledger().timestamp();
    let mut unlocked = 0i128;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if lock.release_timestamp <= now {
            unlocked = checked_add_i128(e, unlocked, lock.amount);
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
    let locks = get_locks(e, token, wallet);
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

    set_locks(e, token, wallet, &new_locks);

    let total_locked = get_total_locked(e, token, wallet);
    set_total_locked(e, token, wallet, checked_sub_i128(e, total_locked, consumed_total));
}

// ---------------------------------------------------------------------------
// ComplianceModule trait implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl ComplianceModule for InitialLockupPeriodModule {
    /// T-REX `moduleTransferAction`: after a P2P transfer, consume expired
    /// lock entries if the transfer ate into the "locked" portion of balance.
    /// Also updates internal balance mirrors for both sender and receiver.
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let total_locked = get_total_locked(e, &token, &from);

        if total_locked > 0 {
            let pre_balance = get_internal_balance(e, &token, &from);
            let pre_free = pre_balance - total_locked;

            if amount > pre_free.max(0) {
                let to_consume = amount - pre_free.max(0);
                update_locked_tokens(e, &token, &from, to_consume);
            }
        }

        let from_bal = get_internal_balance(e, &token, &from);
        set_internal_balance(e, &token, &from, checked_sub_i128(e, from_bal, amount));

        let to_bal = get_internal_balance(e, &token, &to);
        set_internal_balance(e, &token, &to, checked_add_i128(e, to_bal, amount));
    }

    /// T-REX `moduleMintAction`: push a new lock entry for the minted amount
    /// and update the internal balance mirror.
    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let period = get_lockup_period(e, &token);
        if period > 0 {
            let mut locks = get_locks(e, &token, &to);
            locks.push_back(LockedTokens {
                amount,
                release_timestamp: e.ledger().timestamp().saturating_add(period),
            });
            set_locks(e, &token, &to, &locks);

            let total = get_total_locked(e, &token, &to);
            set_total_locked(e, &token, &to, checked_add_i128(e, total, amount));
        }

        let current = get_internal_balance(e, &token, &to);
        set_internal_balance(e, &token, &to, checked_add_i128(e, current, amount));
    }

    /// T-REX `moduleBurnAction`: panics if the burn would consume
    /// still-locked (non-expired) tokens, otherwise cleans up expired entries
    /// and decrements the internal balance mirror.
    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let total_locked = get_total_locked(e, &token, &from);

        if total_locked > 0 {
            let pre_balance = get_internal_balance(e, &token, &from);
            let mut free_amount = pre_balance - total_locked;

            if free_amount < amount {
                let locks = get_locks(e, &token, &from);
                free_amount += calculate_unlocked_amount(e, &locks);
            }

            assert!(
                free_amount >= amount,
                "InitialLockupPeriodModule: insufficient unlocked balance for burn"
            );

            let pre_free = pre_balance - total_locked;
            if amount > pre_free.max(0) {
                let to_consume = amount - pre_free.max(0);
                update_locked_tokens(e, &token, &from, to_consume);
            }
        }

        let current = get_internal_balance(e, &token, &from);
        set_internal_balance(e, &token, &from, checked_sub_i128(e, current, amount));
    }

    /// T-REX `moduleCheck`: allow transfer only if free balance >= amount.
    /// Free balance = `internalBalance - totalLocked + sum(expired_entries)`.
    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        assert!(
            hooks_verified(e),
            "InitialLockupPeriodModule: not armed — call verify_hook_wiring() after wiring hooks \
             [CanTransfer, Created, Transferred, Destroyed]"
        );
        if amount < 0 {
            return false;
        }

        let total_locked = get_total_locked(e, &token, &from);
        if total_locked == 0 {
            return true;
        }

        let balance = get_internal_balance(e, &token, &from);
        let free = balance - total_locked;

        if free >= amount {
            return true;
        }

        let locks = get_locks(e, &token, &from);
        let unlocked = calculate_unlocked_amount(e, &locks);
        (free + unlocked) >= amount
    }

    /// Minting is always allowed — it creates the lock entries, not blocks
    /// them.
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
