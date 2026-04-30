use soroban_sdk::{contracttype, Address, Env, Vec};

use super::{emit_country_restricted, emit_country_unrestricted};
use crate::rwa::compliance::modules::{
    storage::{country_code, get_irs_country_data_entries},
    MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

#[contracttype]
#[derive(Clone)]
pub enum CountryRestrictStorageKey {
    /// Per-(token, country) restriction membership entry.
    RestrictedCountry(Address, u32),
}

// ################## QUERY STATE ##################

/// Returns whether the given country is on the restriction list for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code.
pub fn is_country_restricted(e: &Env, token: &Address, country: u32) -> bool {
    let key = CountryRestrictStorageKey::RestrictedCountry(token.clone(), country);
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Returns `false` if `account` has any restricted country in the IRS for
/// `token`, and `true` otherwise.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The account whose country data is checked.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet`]
///   - When no IRS has been configured for `token`.
///
/// # Cross-Contract Calls
///
/// Calls the IRS to resolve country data for `account`.
pub fn can_receive(e: &Env, account: &Address, token: &Address) -> bool {
    let entries = get_irs_country_data_entries(e, token, account);
    for entry in entries.iter() {
        if is_country_restricted(e, token, country_code(&entry.country)) {
            return false;
        }
    }
    true
}

/// Returns `true` if the transfer recipient has no restricted country.
///
/// Country restriction checks are recipient-based, so the sender and amount are
/// intentionally ignored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `_from` - The sender address.
/// * `to` - The recipient address.
/// * `_amount` - The transfer amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet`]
///   - When no IRS has been configured for `token`.
pub fn can_transfer(
    e: &Env,
    _from: &Address,
    to: &Address,
    _amount: i128,
    token: &Address,
) -> bool {
    can_receive(e, to, token)
}

/// Returns `true` if the mint recipient has no restricted country.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient address.
/// * `_amount` - The minted amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet`]
///   - When no IRS has been configured for `token`.
pub fn can_create(e: &Env, to: &Address, _amount: i128, token: &Address) -> bool {
    can_receive(e, to, token)
}

// ################## CHANGE STATE ##################

/// Records a country as restricted in persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to restrict.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_country_restricted(e: &Env, token: &Address, country: u32) {
    let key = CountryRestrictStorageKey::RestrictedCountry(token.clone(), country);
    e.storage().persistent().set(&key, &());
}

/// Removes a country from the restriction list in persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to unrestrict.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn remove_country_restricted(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .remove(&CountryRestrictStorageKey::RestrictedCountry(token.clone(), country));
}

/// Adds a country to the restriction list for `token`.
///
/// Records the membership entry and emits
/// [`CountryRestricted`](super::CountryRestricted) if the country was not
/// already restricted.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to restrict.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn add_country_restriction(e: &Env, token: &Address, country: u32) {
    if !is_country_restricted(e, token, country) {
        set_country_restricted(e, token, country);
        emit_country_restricted(e, token, country);
    }
}

/// Removes a country from the restriction list for `token`.
///
/// Deletes the membership entry and emits
/// [`CountryUnrestricted`](super::CountryUnrestricted) if the country was
/// currently restricted.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to unrestrict.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn remove_country_restriction(e: &Env, token: &Address, country: u32) {
    if is_country_restricted(e, token, country) {
        remove_country_restricted(e, token, country);
        emit_country_unrestricted(e, token, country);
    }
}

/// Adds multiple countries to the restriction list in a single call.
///
/// Emits [`CountryRestricted`] for each country added.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `countries` - The country codes to restrict.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn batch_restrict_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        add_country_restriction(e, token, country);
    }
}

/// Removes multiple countries from the restriction list in a single call.
///
/// Emits [`CountryUnrestricted`] for each country removed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `countries` - The country codes to unrestrict.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn batch_unrestrict_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        remove_country_restriction(e, token, country);
    }
}
