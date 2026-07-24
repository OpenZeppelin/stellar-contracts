//! Initial-lockup-period compliance module — Stellar port of T-REX
//! [`InitialLockupPeriodModule.sol`][trex-src].
//!
//! Enforces a lockup window on tokens received through primary issuance
//! (mints). Every mint creates a lock entry that releases after the
//! configured per-token period; transfers and burns are allowed only up to
//! the sender's unlocked holdings. Tokens received through peer-to-peer
//! transfers are never locked.
//!
//! The upstream Solidity module reads the sender's live balance from the
//! token contract during hook execution. Soroban prohibits reentrancy, and
//! these hooks run while the token contract is still on the call stack, so a
//! cross-contract `balance()` call is not possible. Instead, the token hands
//! each hook an [`crate::rwa::compliance::AccountSnapshot`] carrying the
//! wallet's pre-operation balance, and this module tracks only the lock
//! schedule: how much of that balance is still locked. The spendable
//! (unlocked) amount is then `balance - locked`, computed afresh on every
//! hook. No balance is mirrored.
//!
//! # Capacity planning
//!
//! A wallet's lock entries live in a single contract-data entry, and their
//! count is capped at [`MAX_LOCKS`]. Expired entries are pruned as mints and
//! spends rewrite the entry, so the count that matters is the number of
//! *concurrently active* locks: the mints a wallet receives within one
//! lockup window, i.e. mint frequency times lockup period. A mint that
//! would push a wallet past the cap is rejected with
//! [`crate::rwa::compliance::modules::ComplianceModuleError::LockBoundExceeded`].
//!
//! Some schedules against the cap:
//!
//! * Monthly dividends, 2-year lockup: at most 24 active locks per wallet. Well
//!   within the cap.
//! * Daily rewards, 1-year lockup: at most ~366 active locks. Also within the
//!   cap.
//! * Daily rewards, 2-year lockup: ~730 active locks. Over the cap: mints to a
//!   steadily rewarded wallet start failing about 17 months in.
//!
//! The cap is a hard deployment requirement: a token expecting any single
//! wallet to receive more than [`MAX_LOCKS`] mints within one lockup window
//! must not use this module as-is; such schedules call for fewer, batched
//! distributions or a shorter lockup period.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/develop/contracts/compliance/modular/modules/InitialLockupPeriodModule.sol

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};
pub use storage::{LockedDetails, LockedTokens};

use crate::rwa::compliance::modules::ComplianceModule;

/// Initial Lockup Period Compliance Module Trait
///
/// The `InitialLockupPeriod` trait extends the [`ComplianceModule`] trait
/// to enforce a lockup window on minted tokens. When this module is
/// registered on a token's modular compliance contract, every mint records
/// a lock entry that releases after the configured period, and transfers
/// are permitted only when the sender holds enough unlocked tokens to cover
/// the amount. Burns are held to the same rule through the burn hook.
///
/// Locks are consumed lazily: expired entries stay on the books until a
/// transfer or burn dips into the locked region, at which point they are
/// consumed oldest-first and removed. Each mint also prunes whatever has
/// expired by then while appending its new lock, so a wallet's stored lock
/// entries stay bounded by its active locks no matter how many mints it
/// receives. Mints are blocked by this module only at that bound: a mint
/// that would push a wallet past [`MAX_LOCKS`] active entries panics
/// rather than record an oversized schedule (see the module docs for
/// sizing guidance).
///
/// The module **maintains its own state**: per-wallet lock entries that
/// record how much of a wallet's balance is still locked. Correct accounting
/// requires the module to be registered on **all** of
/// [`crate::rwa::compliance::ComplianceHook::Transferred`],
/// [`crate::rwa::compliance::ComplianceHook::Created`], and
/// [`crate::rwa::compliance::ComplianceHook::Destroyed`]: `Created` appends
/// a lock on every mint, while `Transferred` and `Destroyed` enforce the
/// lockup (by panicking when the spend exceeds the unlocked holdings) and
/// consume expired locks as the wallet spends, keeping the recorded total
/// in step with what the wallet actually holds. Missing a hook leaves stale
/// locks on the books. Privileged transfers are not rejected: a forced
/// transfer (seizure) consumes locks oldest-first, expired or not, so the
/// schedule shrinks with the seized tokens, while a recovery migrates the
/// consumed entries to the destination wallet with their release times
/// preserved, so the investor's remaining lockup follows the balance onto
/// the new wallet.
///
/// The wallet's balance is never mirrored: the token passes it into each
/// hook via [`crate::rwa::compliance::AccountSnapshot`], and the module
/// derives the spendable amount as `balance - still_locked` on the spot. A
/// token that registers this module at launch needs nothing more. A token
/// whose holders already own locked allocations can seed those locks before
/// binding the module, through a one-shot preset phase:
/// [`InitialLockupPeriod::preset_locks`] records each wallet's pre-existing
/// locks, and [`InitialLockupPeriod::mark_preset_completed`] permanently
/// closes the phase. Closing it is what makes the seeding trustworthy:
/// afterwards no one, the operator included, can rewrite a wallet's locks by
/// hand. Holders with no pre-existing locks need no preset at all, since
/// tokens received through transfers are never locked.
///
/// Locks are tracked per wallet (not per identity) and per-`token`, so a
/// single module contract can serve multiple tokens with independent
/// lockup configurations.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait InitialLockupPeriod: ComplianceModule {
    /// Sets the lockup period for `token`, in ledgers. Affects only locks
    /// created by subsequent mints; existing lock entries keep their
    /// original release times. A period of `0` disables locking for future
    /// mints.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup period is being configured.
    /// * `period` - The lockup duration in ledgers.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["lockup_period_set", token: Address]`
    /// * data - `[period: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::set_lockup_period`]
    /// for the implementation.
    fn set_lockup_period(e: &Env, token: Address, period: u32, operator: Address);

    /// Pre-seeds the lock entries for `wallet` under `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup state is being seeded.
    /// * `wallet` - The wallet whose locks are being seeded.
    /// * `locks` - The lock entries to record for `wallet`.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When any lock amount is negative.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::PresetAlreadyCompleted`] -
    ///   When the preset phase has already been finalized.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::LockBoundExceeded`] -
    ///   When `locks` holds more than [`MAX_LOCKS`] entries.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::MathOverflow`] -
    ///   When summing the lock amounts overflows.
    ///
    /// # Events
    ///
    /// * topics - `["lockup_state_preset", token: Address, wallet: Address]`
    /// * data - `[total_locked: i128]`
    ///
    /// # Notes
    ///
    /// * Intended for registering this module on a token whose holders must
    ///   start with locks already in place; only callable before
    ///   [`InitialLockupPeriod::mark_preset_completed`]. Each wallet's balance
    ///   is read from the token on every hook, so only the locks are seeded.
    /// * No default implementation is provided because this is a privileged
    ///   operation that requires custom access control. Access control should
    ///   be enforced on `operator` before calling [`storage::preset_locks`] for
    ///   the implementation.
    fn preset_locks(
        e: &Env,
        token: Address,
        wallet: Address,
        locks: Vec<LockedTokens>,
        operator: Address,
    );

    /// Finalizes the preset phase for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose preset phase is being finalized.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["preset_completed", token: Address]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// * After this call, any further preset attempts panic.
    /// * No default implementation is provided because this is a privileged
    ///   operation that requires custom access control. Access control should
    ///   be enforced on `operator` before calling
    ///   [`storage::mark_preset_completed`] for the implementation.
    fn mark_preset_completed(e: &Env, token: Address, operator: Address);

    /// Returns the configured lockup period for `token`, in ledgers.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn get_lockup_period(e: &Env, token: Address) -> u32 {
        storage::get_lockup_period(e, &token)
    }

    /// Returns the lock entries and their aggregate tracked for `wallet`
    /// under `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `wallet` - The wallet address.
    fn get_locked_details(e: &Env, token: Address, wallet: Address) -> LockedDetails {
        storage::get_locked_details(e, &token, &wallet)
    }

    /// Returns the amount still locked for `wallet` under `token`: the
    /// aggregate of lock entries whose release time has not yet passed.
    ///
    /// This module tracks locks, not balances. Subtract this from the wallet's
    /// token balance for the spendable amount: `unlocked = balance -
    /// locked_amount`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `wallet` - The wallet address.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::get_locked_amount`] errors.
    fn get_locked_amount(e: &Env, token: Address, wallet: Address) -> i128 {
        storage::get_locked_amount(e, &token, &wallet)
    }

    /// Returns `true` when the preset phase for `token` has been finalized.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn is_preset_completed(e: &Env, token: Address) -> bool {
        storage::is_preset_completed(e, &token)
    }
}

// ################## CONSTANTS ##################

/// Upper bound on the lock entries stored per `(token, wallet)` pair.
///
/// Each entry serializes to roughly 80 bytes of XDR and the ledger caps a
/// single contract-data entry at 64 KiB (`contract_data_entry_size_bytes`),
/// which fits about 800 entries. 512 keeps the entry comfortably below
/// that ceiling while accommodating dense issuance schedules: daily mints
/// under a one-year lockup peak at ~366 active entries. See the module
/// docs for capacity planning.
pub const MAX_LOCKS: u32 = 512;

// ################## EVENTS ##################

/// Emitted when the lockup period for a token is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub period: u32,
}

/// Emits a [`LockupPeriodSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose lockup period changed.
/// * `period` - The new lockup duration in ledgers.
pub fn emit_lockup_period_set(e: &Env, token: &Address, period: u32) {
    LockupPeriodSet { token: token.clone(), period }.publish(e);
}

/// Emitted when a wallet's locks are pre-seeded during the migration preset
/// phase.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupStatePreset {
    #[topic]
    pub token: Address,
    #[topic]
    pub wallet: Address,
    pub total_locked: i128,
}

/// Emits a [`LockupStatePreset`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose lockup state changed.
/// * `wallet` - The wallet whose locks were seeded.
/// * `total_locked` - The aggregate locked amount recorded for `wallet`.
pub fn emit_lockup_state_preset(e: &Env, token: &Address, wallet: &Address, total_locked: i128) {
    LockupStatePreset { token: token.clone(), wallet: wallet.clone(), total_locked }.publish(e);
}

/// Emitted when the preset phase for a token is finalized.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetCompleted {
    #[topic]
    pub token: Address,
}

/// Emits a [`PresetCompleted`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose preset phase was finalized.
pub fn emit_preset_completed(e: &Env, token: &Address) {
    PresetCompleted { token: token.clone() }.publish(e);
}
