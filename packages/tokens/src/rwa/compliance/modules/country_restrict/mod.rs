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

use soroban_sdk::{contractevent, contracttrait, Address, Env, String, Vec};

use super::storage::{get_compliance_address, module_name};

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

/// Country restriction compliance module trait.
///
/// This trait defines the contract-facing API for the country restriction
/// module. Low-level state changes live in [`storage`]. Privileged methods have
/// no default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait CountryRestrict {
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

    /// Country restriction does not track transfer state.
    ///
    /// # Arguments
    ///
    /// * `_e` - Access to the Soroban environment.
    /// * `_from` - The sender address.
    /// * `_to` - The recipient address.
    /// * `_amount` - The transfer amount.
    /// * `_token` - The token address.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// Country restriction does not track mint state.
    ///
    /// # Arguments
    ///
    /// * `_e` - Access to the Soroban environment.
    /// * `_to` - The recipient address.
    /// * `_amount` - The minted amount.
    /// * `_token` - The token address.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// Country restriction does not track burn state.
    ///
    /// # Arguments
    ///
    /// * `_e` - Access to the Soroban environment.
    /// * `_from` - The holder address.
    /// * `_amount` - The burned amount.
    /// * `_token` - The token address.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Returns `true` if the transfer recipient has no restricted country.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The sender address.
    /// * `to` - The recipient address.
    /// * `amount` - The transfer amount.
    /// * `token` - The token address.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        storage::can_transfer(e, &from, &to, amount, &token)
    }

    /// Returns `true` if the mint recipient has no restricted country.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The recipient address.
    /// * `amount` - The minted amount.
    /// * `token` - The token address.
    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        storage::can_create(e, &to, amount, &token)
    }

    /// Returns this module's display name.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn name(e: &Env) -> String {
        module_name(e, "CountryRestrictModule")
    }

    /// Returns the compliance contract address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    /// Sets the compliance contract address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `compliance` - The compliance contract address.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// bootstrap operation that requires custom access control.
    fn set_compliance_address(e: &Env, compliance: Address);
}
