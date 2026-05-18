//! Time-windowed transfer-limits compliance module — Stellar port of T-REX
//! [`TimeTransfersLimitsModule.sol`][trex-src].
//!
//! Limits transfer volume within configurable time windows, tracking counters
//! per **identity** (not per wallet).
//! The Stellar module keeps per-identity transfer counters updated by transfer
//! hooks so limit checks do not need to query token transfer history.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeTransfersLimitsModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

pub use crate::rwa::compliance::modules::time_transfers_limits::storage::{Limit, TransferCounter};
use crate::rwa::compliance::{modules::ComplianceModule, ComplianceHook};

pub const MAX_LIMITS_PER_TOKEN: u32 = 4;

/// Time-window transfer limits compliance module trait.
///
/// This trait defines the contract-facing API for the time transfer limits
/// module. Low-level state changes live in [`storage`]. Privileged methods have
/// no default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait TimeTransfersLimits: ComplianceModule {
    /// Configures the Identity Registry Storage contract for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose IRS is being configured.
    /// * `irs` - The Identity Registry Storage contract address.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address);

    /// Sets or updates a time-window transfer limit for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limit is updated.
    /// * `limit` - The time-window limit.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::set_time_transfer_limit`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["time_transfer_limit_updated", token: Address]`
    /// * data - `[limit: Limit]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit);

    /// Sets or updates multiple time-window transfer limits for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are updated.
    /// * `limits` - The time-window limits.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::batch_set_time_transfer_limit`] errors.
    ///
    /// # Events
    ///
    /// For each configured limit:
    /// * topics - `["time_transfer_limit_updated", token: Address]`
    /// * data - `[limit: Limit]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>);

    /// Removes a time-window transfer limit for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limit is removed.
    /// * `limit_time` - The time-window duration to remove.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::remove_time_transfer_limit`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["time_transfer_limit_removed", token: Address]`
    /// * data - `[limit_time: u64]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64);

    /// Removes multiple time-window transfer limits for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are removed.
    /// * `limit_times` - The time-window durations to remove.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::batch_remove_time_transfer_limit`] errors.
    ///
    /// # Events
    ///
    /// For each removed limit:
    /// * topics - `["time_transfer_limit_removed", token: Address]`
    /// * data - `[limit_time: u64]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>);

    /// Pre-seeds a transfer counter for an identity and time window.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose counter is pre-seeded.
    /// * `identity` - The on-chain identity address.
    /// * `limit_time` - The time-window duration in seconds.
    /// * `counter` - The pre-seeded counter.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::pre_set_transfer_counter`] errors.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn pre_set_transfer_counter(
        e: &Env,
        token: Address,
        identity: Address,
        limit_time: u64,
        counter: TransferCounter,
    );

    /// Returns configured time-window transfer limits for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are queried.
    fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<Limit> {
        storage::get_limits(e, &token)
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

/// Emitted when a time-window limit is added or updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitUpdated {
    #[topic]
    pub token: Address,
    pub limit: Limit,
}

/// Emits a [`TimeTransferLimitUpdated`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose limit was updated.
/// * `limit` - The configured limit.
pub fn emit_time_transfer_limit_updated(e: &Env, token: &Address, limit: &Limit) {
    TimeTransferLimitUpdated { token: token.clone(), limit: limit.clone() }.publish(e);
}

/// Emitted when a time-window limit is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitRemoved {
    #[topic]
    pub token: Address,
    pub limit_time: u64,
}

/// Emits a [`TimeTransferLimitRemoved`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose limit was removed.
/// * `limit_time` - The removed time-window duration.
pub fn emit_time_transfer_limit_removed(e: &Env, token: &Address, limit_time: u64) {
    TimeTransferLimitRemoved { token: token.clone(), limit_time }.publish(e);
}
