//! Supply cap compliance module — Stellar port of T-REX
//! [`SupplyLimitModule.sol`][trex-src].
//!
//! Caps the total number of tokens that can be minted for a given token.
//! The EVM T-REX module reads total supply from the token contract directly;
//! this Stellar module keeps an internal supply mirror updated by create and
//! destroy hooks so cap checks remain local to the module.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/SupplyLimitModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::{modules::ComplianceModule, ComplianceHook};

/// Supply limit compliance module trait.
///
/// This trait defines the contract-facing API for the supply limit module.
/// Low-level state changes live in [`storage`]. Privileged methods have no
/// default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait SupplyLimit: ComplianceModule {
    /// Configures the supply limit for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose supply limit is configured.
    /// * `limit` - The supply cap.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::configure_supply_limit`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["supply_limit_set", token: Address]`
    /// * data - `[limit: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn set_supply_limit(e: &Env, token: Address, limit: i128);

    /// Pre-seeds the internal supply counter for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose supply counter is pre-seeded.
    /// * `supply` - The pre-seeded supply value.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::pre_set_supply`] errors.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn pre_set_supply(e: &Env, token: Address, supply: i128);

    /// Returns the supply limit for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose supply limit is queried.
    fn get_supply_limit(e: &Env, token: Address) -> i128 {
        storage::get_supply_limit(e, &token)
    }

    /// Returns the internal supply counter for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose internal supply is queried.
    fn get_internal_supply(e: &Env, token: Address) -> i128 {
        storage::get_internal_supply(e, &token)
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

/// Emitted when a token's supply cap is configured or changed.
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
/// * `token` - The token whose supply limit was configured.
/// * `limit` - The configured supply cap.
pub fn emit_supply_limit_set(e: &Env, token: &Address, limit: i128) {
    SupplyLimitSet { token: token.clone(), limit }.publish(e);
}
