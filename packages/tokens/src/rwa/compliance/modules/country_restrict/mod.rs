//! Country restriction compliance module — Stellar port of T-REX
//! [`CountryRestrictModule.sol`][trex-src].
//!
//! Recipients whose identity has a country code on the restriction list are
//! blocked from receiving tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryRestrictModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use super::ComplianceModule;

/// Country restriction compliance module trait.
///
/// This trait defines the contract-facing API for the country restriction
/// module. Low-level state changes live in [`storage`]. Privileged methods have
/// no default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait CountryRestrict: ComplianceModule {
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
    /// [`super::storage::set_irs_address`].
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address);

    /// Adds a country to the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `country` - The ISO 3166-1 numeric country code to restrict.
    ///
    /// # Events
    ///
    /// Emits [`CountryRestricted`].
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::add_country_restriction`].
    fn add_country_restriction(e: &Env, token: Address, country: u32);

    /// Removes a country from the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `country` - The ISO 3166-1 numeric country code to unrestrict.
    ///
    /// # Events
    ///
    /// Emits [`CountryUnrestricted`].
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::remove_country_restriction`].
    fn remove_country_restriction(e: &Env, token: Address, country: u32);

    /// Adds multiple countries to the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to restrict.
    ///
    /// # Events
    ///
    /// Emits [`CountryRestricted`] for each country.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::batch_restrict_countries`].
    fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>);

    /// Removes multiple countries from the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to unrestrict.
    ///
    /// # Events
    ///
    /// Emits [`CountryUnrestricted`] for each country.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::batch_unrestrict_countries`].
    fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>);

    /// Returns `true` if `country` is restricted for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is queried.
    /// * `country` - The ISO 3166-1 numeric country code to check.
    fn is_country_restricted(e: &Env, token: Address, country: u32) -> bool {
        storage::is_country_restricted(e, &token, country)
    }
}

/// Emitted when a country is added to the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryRestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emitted when a country is removed from the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnrestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emits a [`CountryRestricted`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose restriction list changed.
/// * `country` - The ISO 3166-1 numeric country code that was restricted.
pub fn emit_country_restricted(e: &Env, token: &Address, country: u32) {
    CountryRestricted { token: token.clone(), country }.publish(e);
}

/// Emits a [`CountryUnrestricted`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose restriction list changed.
/// * `country` - The ISO 3166-1 numeric country code that was removed.
pub fn emit_country_unrestricted(e: &Env, token: &Address, country: u32) {
    CountryUnrestricted { token: token.clone(), country }.publish(e);
}
