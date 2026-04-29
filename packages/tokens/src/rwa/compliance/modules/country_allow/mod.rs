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

use soroban_sdk::{contractevent, contracttrait, Address, Env, String, Vec};

use super::storage::{get_compliance_address, module_name};

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

/// Country allowlist compliance module trait.
///
/// This trait defines the contract-facing API for the country allowlist module.
/// Low-level state changes live in [`storage`]. Privileged methods have no
/// default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait CountryAllow {
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

    /// Adds a country to the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `country` - The ISO 3166-1 numeric country code to allow.
    ///
    /// # Events
    ///
    /// Emits [`CountryAllowed`].
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
    /// # Events
    ///
    /// Emits [`CountryUnallowed`].
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
    /// # Events
    ///
    /// Emits [`CountryAllowed`] for each country.
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
    /// # Events
    ///
    /// Emits [`CountryUnallowed`] for each country.
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

    /// Country allowlist does not track transfer state.
    ///
    /// # Arguments
    ///
    /// * `_e` - Access to the Soroban environment.
    /// * `_from` - The sender address.
    /// * `_to` - The recipient address.
    /// * `_amount` - The transfer amount.
    /// * `_token` - The token address.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// Country allowlist does not track mint state.
    ///
    /// # Arguments
    ///
    /// * `_e` - Access to the Soroban environment.
    /// * `_to` - The recipient address.
    /// * `_amount` - The minted amount.
    /// * `_token` - The token address.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// Country allowlist does not track burn state.
    ///
    /// # Arguments
    ///
    /// * `_e` - Access to the Soroban environment.
    /// * `_from` - The holder address.
    /// * `_amount` - The burned amount.
    /// * `_token` - The token address.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Returns `true` if the transfer recipient has at least one allowed
    /// country.
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

    /// Returns `true` if the mint recipient has at least one allowed country.
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
        module_name(e, "CountryAllowModule")
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
