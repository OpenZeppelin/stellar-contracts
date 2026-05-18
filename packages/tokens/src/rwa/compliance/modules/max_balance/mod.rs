//! Max balance compliance module — Stellar port of T-REX
//! [`MaxBalanceModule.sol`][trex-src].
//!
//! Tracks effective balances per **identity** (not per wallet), enforcing a
//! per-token cap.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::{modules::ComplianceModule, ComplianceHook};

/// Max balance compliance module trait.
///
/// This trait defines the contract-facing API for the max balance module.
/// Low-level state changes live in [`storage`]. Privileged methods have no
/// default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait MaxBalance: ComplianceModule {
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
    /// operation that requires custom access control. Access control should be
    /// enforced before calling
    /// [`crate::rwa::compliance::modules::storage::set_irs_address`].
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address);

    /// Configures the per-identity max balance for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose max balance is configured.
    /// * `max` - The maximum balance per identity.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::configure_max_balance`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["max_balance_set", token: Address]`
    /// * data - `[max_balance: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn set_max_balance(e: &Env, token: Address, max: i128);

    /// Pre-seeds the tracked balance for an identity.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose identity balance is pre-seeded.
    /// * `identity` - The on-chain identity address.
    /// * `balance` - The pre-seeded balance.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::pre_set_identity_balance`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["id_balance_pre_set", token: Address, identity: Address]`
    /// * data - `[balance: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn pre_set_identity_balance(e: &Env, token: Address, identity: Address, balance: i128);

    /// Pre-seeds tracked balances for multiple identities.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose identity balances are pre-seeded.
    /// * `identities` - Identity addresses.
    /// * `balances` - Corresponding balances.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::batch_pre_set_identity_balances`] errors.
    ///
    /// # Events
    ///
    /// For each identity balance pre-seeded:
    /// * topics - `["id_balance_pre_set", token: Address, identity: Address]`
    /// * data - `[balance: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn batch_pre_set_identity_balances(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    );

    /// Returns the per-identity balance cap for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose max balance is queried.
    fn get_max_balance(e: &Env, token: Address) -> i128 {
        storage::get_max_balance(e, &token)
    }

    /// Returns the tracked balance for `identity` on `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose identity balance is queried.
    /// * `identity` - The on-chain identity address.
    fn get_investor_balance(e: &Env, token: Address, identity: Address) -> i128 {
        storage::get_id_balance(e, &token, &identity)
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

/// Emitted when a token's per-identity balance cap is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBalanceSet {
    #[topic]
    pub token: Address,
    pub max_balance: i128,
}

/// Emits a [`MaxBalanceSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose max balance was configured.
/// * `max_balance` - The configured per-identity balance cap.
pub fn emit_max_balance_set(e: &Env, token: &Address, max_balance: i128) {
    MaxBalanceSet { token: token.clone(), max_balance }.publish(e);
}

/// Emitted when an identity balance is pre-seeded via `pre_set_module_state`.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IDBalancePreSet {
    #[topic]
    pub token: Address,
    pub identity: Address,
    pub balance: i128,
}

/// Emits an [`IDBalancePreSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose identity balance was pre-seeded.
/// * `identity` - The identity whose balance was pre-seeded.
/// * `balance` - The pre-seeded balance.
pub fn emit_id_balance_pre_set(e: &Env, token: &Address, identity: &Address, balance: i128) {
    IDBalancePreSet { token: token.clone(), identity: identity.clone(), balance }.publish(e);
}
