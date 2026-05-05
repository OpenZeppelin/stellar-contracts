//! Country allowlist compliance module — Stellar port of T-REX
//! [`CountryAllowModule.sol`][trex-src].
//!
//! Only recipients whose identity has at least one country code in the
//! allowlist may receive tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryAllowModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::modules::ComplianceModule;

/// Country allowlist compliance module trait.
///
/// This trait defines the contract-facing API for the country allowlist module.
/// Low-level state changes live in [`storage`]. Privileged methods have no
/// default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait CountryAllow: ComplianceModule {
    /// Configures the Identity Registry Storage contract for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose IRS is being configured.
    /// * `irs` - The Identity Registry Storage contract address.
    ///
    /// # Errors
    ///
    /// Implementations should fail if the caller is not authorized to configure
    /// the IRS address.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling
    /// [`crate::rwa::compliance::modules::storage::set_irs_address`].
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address);

    /// Adds a country to the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `country` - The ISO 3166-1 numeric country code to allow.
    ///
    /// # Errors
    ///
    /// Implementations should fail if the caller is not authorized to update
    /// the allowlist.
    ///
    /// # Events
    ///
    /// * topics - `["country_allowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::add_allowed_country`].
    fn add_allowed_country(e: &Env, token: Address, country: u32);

    /// Removes a country from the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `country` - The ISO 3166-1 numeric country code to remove.
    ///
    /// # Errors
    ///
    /// Implementations should fail if the caller is not authorized to update
    /// the allowlist.
    ///
    /// # Events
    ///
    /// * topics - `["country_unallowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::remove_allowed_country`].
    fn remove_allowed_country(e: &Env, token: Address, country: u32);

    /// Adds multiple countries to the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to allow.
    ///
    /// # Errors
    ///
    /// Implementations should fail if the caller is not authorized to update
    /// the allowlist.
    ///
    /// # Events
    ///
    /// For each country newly added to the allowlist:
    /// * topics - `["country_allowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::batch_allow_countries`].
    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>);

    /// Removes multiple countries from the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to remove.
    ///
    /// # Errors
    ///
    /// Implementations should fail if the caller is not authorized to update
    /// the allowlist.
    ///
    /// # Events
    ///
    /// For each country removed from the allowlist:
    /// * topics - `["country_unallowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::batch_disallow_countries`].
    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>);

    /// Returns `true` if `country` is allowed for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is queried.
    /// * `country` - The ISO 3166-1 numeric country code to check.
    fn is_country_allowed(e: &Env, token: Address, country: u32) -> bool {
        storage::is_country_allowed(e, &token, country)
    }
}

/// Emitted when a country is added to the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryAllowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emitted when a country is removed from the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnallowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emits a [`CountryAllowed`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose allowlist changed.
/// * `country` - The ISO 3166-1 numeric country code that was allowed.
pub fn emit_country_allowed(e: &Env, token: &Address, country: u32) {
    CountryAllowed { token: token.clone(), country }.publish(e);
}

/// Emits a [`CountryUnallowed`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose allowlist changed.
/// * `country` - The ISO 3166-1 numeric country code that was removed.
pub fn emit_country_unallowed(e: &Env, token: &Address, country: u32) {
    CountryUnallowed { token: token.clone(), country }.publish(e);
}
