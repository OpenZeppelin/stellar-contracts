//! Supply-limit compliance module — Stellar port of T-REX
//! [`SupplyLimitModule.sol`][trex-src].
//!
//! Enforces a global supply cap per token. The module maintains an
//! internal supply counter that increments on every mint hook and
//! decrements on every burn hook. When a mint would push the running
//! supply above the configured limit, the operation is blocked.
//!
//! Unlike the Solidity reference (which reads the token's
//! `totalSupply()` at validation time), this port keeps its own counter.
//! That makes the supply check a single per-token storage read instead of
//! a cross-contract call, but it requires the module to be registered on
//! [`crate::rwa::compliance::ComplianceHook::Created`] and
//! [`crate::rwa::compliance::ComplianceHook::Destroyed`] so the counter
//! stays in sync with reality.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/SupplyLimitModule.sol

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env};

use crate::rwa::compliance::modules::ComplianceModule;

/// Supply Limit Compliance Module Trait
///
/// The `SupplyLimit` trait extends the [`ComplianceModule`] trait to
/// enforce a per-token cap on the circulating supply. When this module is
/// registered on a token's modular compliance contract, mints are blocked
/// once the running supply would exceed the configured limit. Transfers
/// are not affected — they move tokens between holders without changing
/// the supply.
///
/// The module **maintains its own state**: it keeps a running counter that
/// increments on the [`crate::rwa::compliance::ComplianceHook::Created`]
/// hook and decrements on the
/// [`crate::rwa::compliance::ComplianceHook::Destroyed`] hook. Correct
/// accounting therefore requires the module to be registered on both hooks
/// (Created/Destroyed): the mint hook both enforces the cap (by panicking)
/// and increments the counter. Missing a hook causes the counter to drift
/// away from the actual on-chain supply.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait SupplyLimit: ComplianceModule {
    /// Sets the supply cap for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose cap is being configured.
    /// * `limit` - The new supply cap.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When `limit` is negative.
    ///
    /// # Events
    ///
    /// * topics - `["supply_limit_set", token: Address]`
    /// * data - `[limit: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::set_supply_limit`] for
    /// the implementation.
    fn set_supply_limit(e: &Env, token: Address, limit: i128, operator: Address);

    /// Returns the configured supply cap for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn get_supply_limit(e: &Env, token: Address) -> i128 {
        storage::get_supply_limit(e, &token)
    }

    /// Returns the running supply counter tracked by this module for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn get_supply_count(e: &Env, token: Address) -> i128 {
        storage::get_supply_count(e, &token)
    }
}

// ################## EVENTS ##################

/// Emitted when the per-token supply cap is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyLimitSet {
    #[topic]
    pub token: Address,
    pub limit: i128,
}

/// Emits a [`SupplyLimitSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose cap changed.
/// * `limit` - The new supply cap.
pub fn emit_supply_limit_set(e: &Env, token: &Address, limit: i128) {
    SupplyLimitSet { token: token.clone(), limit }.publish(e);
}

/// Emitted whenever the tracked supply counter for a token changes.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyCountUpdated {
    #[topic]
    pub token: Address,
    pub supply: i128,
}

/// Emits a [`SupplyCountUpdated`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose tracked supply changed.
/// * `supply` - The new tracked supply.
pub fn emit_supply_count_updated(e: &Env, token: &Address, supply: i128) {
    SupplyCountUpdated { token: token.clone(), supply }.publish(e);
}
