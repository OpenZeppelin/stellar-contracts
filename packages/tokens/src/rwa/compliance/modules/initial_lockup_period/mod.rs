//! Initial lockup period compliance module — Stellar port of T-REX
//! [`TimeExchangeLimitsModule.sol`][trex-src].
//!
//! Enforces a lockup period for all investors whenever they receive tokens
//! through primary emissions (mints). Tokens received via peer-to-peer
//! transfers are **not** subject to lockup restrictions.
//! The Stellar module keeps internal balance and lock mirrors updated by
//! create, transfer, and destroy hooks so transfer checks can remain local to
//! the module.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeExchangeLimitsModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

pub use crate::rwa::compliance::modules::initial_lockup_period::storage::LockedTokens;
use crate::rwa::compliance::{modules::ComplianceModule, ComplianceHook};

/// Initial lockup period compliance module trait.
///
/// This trait defines the contract-facing API for the initial lockup module.
/// Low-level state changes live in [`storage`]. Privileged methods have no
/// default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait InitialLockupPeriod: ComplianceModule {
    /// Configures the lockup period for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup period is configured.
    /// * `lockup_seconds` - The lockup duration in seconds.
    ///
    /// # Events
    ///
    /// * topics - `["lockup_period_set", token: Address]`
    /// * data - `[lockup_seconds: u64]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn set_lockup_period(e: &Env, token: Address, lockup_seconds: u64);

    /// Pre-seeds lockup state for `wallet`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup state is pre-seeded.
    /// * `wallet` - The wallet address.
    /// * `balance` - The wallet balance mirror.
    /// * `locks` - Existing lock entries.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::pre_set_lockup_state`] errors.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn pre_set_lockup_state(
        e: &Env,
        token: Address,
        wallet: Address,
        balance: i128,
        locks: Vec<LockedTokens>,
    );

    /// Returns the lockup period for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup period is queried.
    fn get_lockup_period(e: &Env, token: Address) -> u64 {
        storage::get_lockup_period(e, &token)
    }

    /// Returns the aggregate locked amount for `wallet` on `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup state is queried.
    /// * `wallet` - The wallet address.
    fn get_total_locked(e: &Env, token: Address, wallet: Address) -> i128 {
        storage::get_total_locked(e, &token, &wallet)
    }

    /// Returns lock entries for `wallet` on `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose lockup state is queried.
    /// * `wallet` - The wallet address.
    fn get_locked_tokens(e: &Env, token: Address, wallet: Address) -> Vec<LockedTokens> {
        storage::get_locks(e, &token, &wallet)
    }

    /// Returns the internal balance mirror for `wallet` on `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose balance mirror is queried.
    /// * `wallet` - The wallet address.
    fn get_internal_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        storage::get_internal_balance(e, &token, &wallet)
    }

    /// Returns the set of compliance hooks this module requires.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        storage::required_hooks(e)
    }

    /// Verifies that this module is registered on every required hook.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::verify_hook_wiring`] errors.
    fn verify_hook_wiring(e: &Env) {
        storage::verify_hook_wiring(e);
    }
}

// ################## EVENTS ##################

/// Emitted when a token's lockup duration is configured or changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub lockup_seconds: u64,
}

/// Emits a [`LockupPeriodSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose lockup period was configured.
/// * `lockup_seconds` - The configured lockup duration in seconds.
pub fn emit_lockup_period_set(e: &Env, token: &Address, lockup_seconds: u64) {
    LockupPeriodSet { token: token.clone(), lockup_seconds }.publish(e);
}
