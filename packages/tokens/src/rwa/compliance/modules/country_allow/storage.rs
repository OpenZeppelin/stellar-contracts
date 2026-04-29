use soroban_sdk::{contracttype, Address, Env, Vec};

use super::{CountryAllowed, CountryUnallowed};
use crate::rwa::compliance::modules::{
    storage::{country_code, get_irs_country_data_entries},
    MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

#[contracttype]
#[derive(Clone)]
pub enum CountryAllowStorageKey {
    /// Per-(token, country) allowlist flag.
    AllowedCountry(Address, u32),
}

// ################## RAW STORAGE ##################

/// Returns whether the given country is on the allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code.
pub fn is_country_allowed(e: &Env, token: &Address, country: u32) -> bool {
    let key = CountryAllowStorageKey::AllowedCountry(token.clone(), country);
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Writes a country's allowed flag to persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to allow.
pub fn set_country_allowed(e: &Env, token: &Address, country: u32) {
    let key = CountryAllowStorageKey::AllowedCountry(token.clone(), country);
    e.storage().persistent().set(&key, &());
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Removes a country from the allowlist in persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to remove.
pub fn remove_country_allowed(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .remove(&CountryAllowStorageKey::AllowedCountry(token.clone(), country));
}

// ################## ACTIONS ##################

/// Adds a country to the allowlist for `token`.
///
/// Writes the flag to storage and emits [`CountryAllowed`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to allow.
pub fn add_allowed_country(e: &Env, token: &Address, country: u32) {
    set_country_allowed(e, token, country);
    CountryAllowed { token: token.clone(), country }.publish(e);
}

/// Removes a country from the allowlist for `token`.
///
/// Deletes the flag from storage and emits [`CountryUnallowed`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to remove.
pub fn remove_allowed_country(e: &Env, token: &Address, country: u32) {
    remove_country_allowed(e, token, country);
    CountryUnallowed { token: token.clone(), country }.publish(e);
}

/// Adds multiple countries to the allowlist in a single call.
///
/// Emits [`CountryAllowed`] for each country added.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `countries` - The country codes to allow.
pub fn batch_allow_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        set_country_allowed(e, token, country);
        CountryAllowed { token: token.clone(), country }.publish(e);
    }
}

/// Removes multiple countries from the allowlist in a single call.
///
/// Emits [`CountryUnallowed`] for each country removed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `countries` - The country codes to remove.
pub fn batch_disallow_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        remove_country_allowed(e, token, country);
        CountryUnallowed { token: token.clone(), country }.publish(e);
    }
}

// ################## COMPLIANCE HOOKS ##################

/// Returns `true` if `account` has at least one allowed country in the IRS for
/// `token`.
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
        if is_country_allowed(e, token, country_code(&entry.country)) {
            return true;
        }
    }
    false
}

/// Returns `true` if the transfer recipient has at least one allowed country.
///
/// Country allowlist checks are recipient-based, so the sender and amount are
/// intentionally ignored.
pub fn can_transfer(
    e: &Env,
    _from: &Address,
    to: &Address,
    _amount: i128,
    token: &Address,
) -> bool {
    can_receive(e, to, token)
}

/// Returns `true` if the mint recipient has at least one allowed country.
pub fn can_create(e: &Env, to: &Address, _amount: i128, token: &Address) -> bool {
    can_receive(e, to, token)
}
