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
//! these hooks run while the token contract is still on the call stack, so
//! a cross-contract `balance()` call is not possible. Instead, this module
//! maintains its own per-wallet balance mirror, updated by the
//! [`crate::rwa::compliance::ComplianceHook::Transferred`],
//! [`crate::rwa::compliance::ComplianceHook::Created`], and
//! [`crate::rwa::compliance::ComplianceHook::Destroyed`] hooks.
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
/// consumed oldest-first and removed. Mints themselves are never blocked by
/// this module: they are the operation that creates locks.
///
/// The module **maintains its own state**: per-wallet lock entries plus a
/// balance mirror that tracks each wallet's token balance. Correct
/// accounting therefore requires the module to be registered on **all** of
/// [`crate::rwa::compliance::ComplianceHook::Transferred`],
/// [`crate::rwa::compliance::ComplianceHook::Created`], and
/// [`crate::rwa::compliance::ComplianceHook::Destroyed`] in addition to the
/// validation hook [`crate::rwa::compliance::ComplianceHook::CanTransfer`].
/// Missing a state-mutating hook causes the mirror to drift from reality.
///
/// The transfer check trusts the mirror alone, never the token's actual
/// balances: a wallet can spend at most its mirrored balance minus
/// whatever is still locked. A token that registers this module at launch
/// needs nothing more, since every wallet's mirror grows from zero through
/// the hooks and never drifts. A token whose holders already own balances
/// starts with every mirror at zero instead, leaving those holders unable
/// to send. For that migration case the trait exposes a one-shot preset
/// phase: the operator copies each holder's real balance (and any
/// pre-existing locks) into the module via
/// [`InitialLockupPeriod::preset_lockup_state`], and, once every holder is
/// seeded, permanently closes the phase with
/// [`InitialLockupPeriod::mark_preset_completed`]. Closing the phase is
/// what makes the seeding trustworthy: afterwards no one, the operator
/// included, can rewrite a wallet's mirrored state by hand.
///
/// Locks are tracked per wallet (not per identity) and per-`token`, so a
/// single module contract can serve multiple tokens with independent
/// lockup configurations.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait InitialLockupPeriod: ComplianceModule {
    /// Sets the lockup period for `token`, in seconds. Affects only locks
    /// created by subsequent mints; existing lock entries keep their
    /// original release times. A period of `0` disables locking for future
    /// mints.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup period is being configured.
    /// * `period` - The lockup duration in seconds.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["lockup_period_set", token: Address]`
    /// * data - `[period: u64]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::set_lockup_period`]
    /// for the implementation.
    fn set_lockup_period(e: &Env, token: Address, period: u64, operator: Address);

    /// Pre-seeds the lockup state for `wallet` under `token`: the balance
    /// mirror and any pre-existing lock entries.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup state is being seeded.
    /// * `wallet` - The wallet whose state is being seeded.
    /// * `balance` - The wallet's token balance to mirror.
    /// * `locks` - The lock entries to record for `wallet`.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When `balance` or any lock amount is negative.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::PresetAlreadyCompleted`] -
    ///   When the preset phase has already been finalized.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::MathOverflow`] -
    ///   When summing the lock amounts overflows.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::LockedAmountExceedsBalance`] -
    ///   When the aggregate locked amount exceeds `balance`.
    ///
    /// # Events
    ///
    /// * topics - `["lockup_state_preset", token: Address, wallet: Address]`
    /// * data - `[balance: i128, total_locked: i128]`
    ///
    /// # Notes
    ///
    /// * Intended for registering this module on a token whose holders must
    ///   start with locks already in place; only callable before
    ///   [`InitialLockupPeriod::mark_preset_completed`].
    /// * No default implementation is provided because this is a privileged
    ///   operation that requires custom access control. Access control should
    ///   be enforced on `operator` before calling
    ///   [`storage::preset_lockup_state`] for the implementation.
    fn preset_lockup_state(
        e: &Env,
        token: Address,
        wallet: Address,
        balance: i128,
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

    /// Returns the configured lockup period for `token`, in seconds.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn get_lockup_period(e: &Env, token: Address) -> u64 {
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

    /// Returns the balance mirror tracked for `wallet` under `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `wallet` - The wallet address.
    fn get_tracked_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        storage::get_tracked_balance(e, &token, &wallet)
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
    /// * refer to [`storage::get_unlocked_balance`] errors.
    fn get_unlocked_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        storage::get_unlocked_balance(e, &token, &wallet)
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

// ################## EVENTS ##################

/// Emitted when the lockup period for a token is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub period: u64,
}

/// Emits a [`LockupPeriodSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose lockup period changed.
/// * `period` - The new lockup duration in seconds.
pub fn emit_lockup_period_set(e: &Env, token: &Address, period: u64) {
    LockupPeriodSet { token: token.clone(), period }.publish(e);
}

/// Emitted when a wallet's lockup state is pre-seeded during the migration
/// preset phase.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupStatePreset {
    #[topic]
    pub token: Address,
    #[topic]
    pub wallet: Address,
    pub balance: i128,
    pub total_locked: i128,
}

/// Emits a [`LockupStatePreset`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose lockup state changed.
/// * `wallet` - The wallet whose state was seeded.
/// * `balance` - The mirrored balance recorded for `wallet`.
/// * `total_locked` - The aggregate locked amount recorded for `wallet`.
pub fn emit_lockup_state_preset(
    e: &Env,
    token: &Address,
    wallet: &Address,
    balance: i128,
    total_locked: i128,
) {
    LockupStatePreset { token: token.clone(), wallet: wallet.clone(), balance, total_locked }
        .publish(e);
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
