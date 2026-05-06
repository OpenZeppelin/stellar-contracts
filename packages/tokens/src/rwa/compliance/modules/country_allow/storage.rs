use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::rwa::compliance::modules::{
    country_allow::{emit_country_allowed, emit_country_unallowed},
    storage::{country_code, get_irs_country_data_entries},
    MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

#[contracttype]
#[derive(Clone)]
pub enum CountryAllowStorageKey {
    /// Per-(token, country) allowlist membership entry.
    AllowedCountry(Address, u32),
}

// ################## QUERY STATE ##################

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
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for `token`.
///
/// # Cross-Contract Calls
///
/// Calls the IRS to resolve country data for `account`.
///
/// [`ComplianceModuleError::IdentityRegistryNotSet`]: crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet
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
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for `token`.
///
/// [`ComplianceModuleError::IdentityRegistryNotSet`]: crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet
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
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for `token`.
///
/// [`ComplianceModuleError::IdentityRegistryNotSet`]: crate::rwa::compliance::modules::ComplianceModuleError::IdentityRegistryNotSet
pub fn can_create(e: &Env, to: &Address, _amount: i128, token: &Address) -> bool {
    can_receive(e, to, token)
}

// ################## CHANGE STATE ##################

/// Records a country as allowed in persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to allow.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_country_allowed(e: &Env, token: &Address, country: u32) {
    let key = CountryAllowStorageKey::AllowedCountry(token.clone(), country);
    e.storage().persistent().set(&key, &());
}

/// Removes a country from the allowlist in persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to remove.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn remove_country_allowed(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .remove(&CountryAllowStorageKey::AllowedCountry(token.clone(), country));
}

/// Adds a country to the allowlist for `token`.
///
/// Records the membership entry and emits
/// [`crate::rwa::compliance::modules::country_allow::CountryAllowed`] if the
/// country was not already allowed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to allow.
///
/// # Events
///
/// * topics - `["country_allowed", token: Address]`
/// * data - `[country: u32]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn add_allowed_country(e: &Env, token: &Address, country: u32) {
    if !is_country_allowed(e, token, country) {
        set_country_allowed(e, token, country);
        emit_country_allowed(e, token, country);
    }
}

/// Removes a country from the allowlist for `token`.
///
/// Deletes the membership entry and emits
/// [`crate::rwa::compliance::modules::country_allow::CountryUnallowed`] if the
/// country was currently allowed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to remove.
///
/// # Events
///
/// * topics - `["country_unallowed", token: Address]`
/// * data - `[country: u32]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn remove_allowed_country(e: &Env, token: &Address, country: u32) {
    if is_country_allowed(e, token, country) {
        remove_country_allowed(e, token, country);
        emit_country_unallowed(e, token, country);
    }
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
///
/// # Events
///
/// For each country newly added to the allowlist:
/// * topics - `["country_allowed", token: Address]`
/// * data - `[country: u32]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn batch_allow_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        add_allowed_country(e, token, country);
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
///
/// # Events
///
/// For each country removed from the allowlist:
/// * topics - `["country_unallowed", token: Address]`
/// * data - `[country: u32]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn batch_disallow_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        remove_allowed_country(e, token, country);
    }
}
