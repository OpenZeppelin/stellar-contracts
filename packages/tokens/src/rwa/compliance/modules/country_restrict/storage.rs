use soroban_sdk::{contracttype, Address, Env, Vec};

use super::{CountryRestricted, CountryUnrestricted};
use crate::rwa::compliance::modules::{
    storage::{country_code, get_irs_country_data_entries},
    MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

#[contracttype]
#[derive(Clone)]
pub enum CountryRestrictStorageKey {
    /// Per-(token, country) restriction flag.
    RestrictedCountry(Address, u32),
}

// ################## RAW STORAGE ##################

/// Returns whether the given country is on the restriction list for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code.
pub fn is_country_restricted(e: &Env, token: &Address, country: u32) -> bool {
    let key = CountryRestrictStorageKey::RestrictedCountry(token.clone(), country);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &bool| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Writes a country's restricted flag to persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to restrict.
pub fn set_country_restricted(e: &Env, token: &Address, country: u32) {
    let key = CountryRestrictStorageKey::RestrictedCountry(token.clone(), country);
    e.storage().persistent().set(&key, &true);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Removes a country from the restriction list in persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to unrestrict.
pub fn remove_country_restricted(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .remove(&CountryRestrictStorageKey::RestrictedCountry(token.clone(), country));
}

// ################## ACTIONS ##################

/// Adds a country to the restriction list for `token`.
///
/// Writes the flag to storage and emits [`CountryRestricted`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to restrict.
pub fn add_country_restriction(e: &Env, token: &Address, country: u32) {
    set_country_restricted(e, token, country);
    CountryRestricted { token: token.clone(), country }.publish(e);
}

/// Removes a country from the restriction list for `token`.
///
/// Deletes the flag from storage and emits [`CountryUnrestricted`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to unrestrict.
pub fn remove_country_restriction(e: &Env, token: &Address, country: u32) {
    remove_country_restricted(e, token, country);
    CountryUnrestricted { token: token.clone(), country }.publish(e);
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
pub fn batch_restrict_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        set_country_restricted(e, token, country);
        CountryRestricted { token: token.clone(), country }.publish(e);
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
pub fn batch_unrestrict_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        remove_country_restricted(e, token, country);
        CountryUnrestricted { token: token.clone(), country }.publish(e);
    }
}

// ################## COMPLIANCE HOOKS ##################

/// Returns `false` if `to` has any restricted country in the IRS for `token`,
/// and `true` otherwise.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient whose country data is checked.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet`]
///   - When no IRS has been configured for `token`.
///
/// # Cross-Contract Calls
///
/// Calls the IRS to resolve country data for `to`.
pub fn can_transfer(e: &Env, to: &Address, token: &Address) -> bool {
    let entries = get_irs_country_data_entries(e, token, to);
    for entry in entries.iter() {
        if is_country_restricted(e, token, country_code(&entry.country)) {
            return false;
        }
    }
    true
}
