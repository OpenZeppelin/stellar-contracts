//! Time-windowed transfer-limits compliance module — Stellar port of T-REX
//! [`TimeTransfersLimitsModule.sol`][trex-src].
//!
//! Enforces cumulative transfer limits within configurable time windows.
//! Each sender identity's outgoing volume is accumulated per window; a
//! transfer is rejected when it alone exceeds a window's cap or when it
//! would push the running counter past it. Counters reset once their
//! window elapses.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeTransfersLimitsModule.sol

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};
pub use storage::{TransferCounter, TransferLimit};

use crate::rwa::compliance::modules::ComplianceModule;

/// Time Transfers Limits Compliance Module Trait
///
/// The `TimeTransfersLimits` trait extends the [`ComplianceModule`] trait
/// to cap the volume an investor can send within rolling time windows. Up
/// to [`MAX_LIMITS`] windows can be configured per token (for example, a
/// daily and a monthly cap); a transfer must satisfy every configured
/// window to pass.
///
/// Counters are tracked per identity (not per wallet), resolved through
/// the token's Identity Registry Storage on every hook call: spreading
/// transfers across wallets controlled by the same on-chain identity does
/// not raise the effective cap. Each counter starts with the first
/// transfer after its window elapsed and resets once the window passes.
///
/// Only outgoing transfers are counted and checked. Mints and burns are
/// exempt. The upstream Solidity module also exempts token agents from
/// the check; this port has no agent concept, so deployments needing
/// exemptions should layer them in their contract's
/// [`ComplianceModule::can_transfer`] implementation.
///
/// The module **maintains its own state**: it accumulates sender volume on
/// every transfer. Correct accounting therefore requires the module to be
/// registered on [`crate::rwa::compliance::ComplianceHook::Transferred`]
/// in addition to the validation hook
/// [`crate::rwa::compliance::ComplianceHook::CanTransfer`]. Missing the
/// state-mutating hook causes the counters to drift from reality.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait TimeTransfersLimits: ComplianceModule {
    /// Configures the Identity Registry Storage contract for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose IRS is being configured.
    /// * `irs` - The Identity Registry Storage contract address.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`crate::rwa::compliance::modules::storage::set_irs_address`] for the
    /// implementation.
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address, operator: Address);

    /// Adds or updates a time-window limit for `token`. A limit with the
    /// same window duration replaces the existing entry; a new window is
    /// appended.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are updated.
    /// * `limit` - The window duration (in seconds) and the volume cap.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When the limit value is negative.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::LimitBoundExceeded`] -
    ///   When appending a new window would exceed [`MAX_LIMITS`].
    ///
    /// # Events
    ///
    /// * topics - `["time_transfer_limit_set", token: Address]`
    /// * data - `[limit_duration: u64, limit_value: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::set_time_transfer_limit`] for the implementation.
    fn set_time_transfer_limit(e: &Env, token: Address, limit: TransferLimit, operator: Address);

    /// Adds or updates multiple time-window limits for `token` in a single
    /// call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are updated.
    /// * `limits` - The limits to add or update.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * refer to [`TimeTransfersLimits::set_time_transfer_limit`] errors.
    ///
    /// # Events
    ///
    /// For each limit:
    /// * topics - `["time_transfer_limit_set", token: Address]`
    /// * data - `[limit_duration: u64, limit_value: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::batch_set_time_transfer_limit`] for the implementation.
    fn batch_set_time_transfer_limit(
        e: &Env,
        token: Address,
        limits: Vec<TransferLimit>,
        operator: Address,
    );

    /// Removes the time-window limit with duration `limit_duration` for
    /// `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are updated.
    /// * `limit_duration` - The window duration (in seconds) to remove.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::LimitNotFound`] -
    ///   When no limit exists for `limit_duration`.
    ///
    /// # Events
    ///
    /// * topics - `["time_transfer_limit_removed", token: Address]`
    /// * data - `[limit_duration: u64]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::remove_time_transfer_limit`] for the implementation.
    fn remove_time_transfer_limit(e: &Env, token: Address, limit_duration: u64, operator: Address);

    /// Removes multiple time-window limits for `token` in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are updated.
    /// * `limit_durations` - The window durations (in seconds) to remove.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * refer to [`TimeTransfersLimits::remove_time_transfer_limit`] errors.
    ///
    /// # Events
    ///
    /// For each removed limit:
    /// * topics - `["time_transfer_limit_removed", token: Address]`
    /// * data - `[limit_duration: u64]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::batch_remove_time_transfer_limit`] for the implementation.
    fn batch_remove_time_transfer_limit(
        e: &Env,
        token: Address,
        limit_durations: Vec<u64>,
        operator: Address,
    );

    /// Returns the time-window limits configured for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose limits are queried.
    fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<TransferLimit> {
        storage::get_time_transfer_limits(e, &token)
    }

    /// Returns the transfer counter tracked for `identity` within the
    /// `limit_duration` window under `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose counter is queried.
    /// * `identity` - The identity address.
    /// * `limit_duration` - The window duration in seconds.
    fn get_transfer_counter(
        e: &Env,
        token: Address,
        identity: Address,
        limit_duration: u64,
    ) -> TransferCounter {
        storage::get_transfer_counter(e, &token, &identity, limit_duration)
    }
}

// ################## CONSTANTS ##################

/// Maximum number of time-window limits that can be configured per token.
/// Every transfer iterates all configured windows, so the bound keeps the
/// per-transfer cost predictable.
pub const MAX_LIMITS: u32 = 8;

// ################## EVENTS ##################

/// Emitted when a time-window limit is added or updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitSet {
    #[topic]
    pub token: Address,
    pub limit_duration: u64,
    pub limit_value: i128,
}

/// Emits a [`TimeTransferLimitSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose limits changed.
/// * `limit_duration` - The window duration in seconds.
/// * `limit_value` - The volume cap for the window.
pub fn emit_time_transfer_limit_set(
    e: &Env,
    token: &Address,
    limit_duration: u64,
    limit_value: i128,
) {
    TimeTransferLimitSet { token: token.clone(), limit_duration, limit_value }.publish(e);
}

/// Emitted when a time-window limit is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitRemoved {
    #[topic]
    pub token: Address,
    pub limit_duration: u64,
}

/// Emits a [`TimeTransferLimitRemoved`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose limits changed.
/// * `limit_duration` - The window duration in seconds that was removed.
pub fn emit_time_transfer_limit_removed(e: &Env, token: &Address, limit_duration: u64) {
    TimeTransferLimitRemoved { token: token.clone(), limit_duration }.publish(e);
}
