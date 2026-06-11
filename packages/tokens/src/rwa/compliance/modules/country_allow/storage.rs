use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::compliance::{
    modules::{
        country_allow::{emit_country_allowed, emit_country_unallowed},
        storage::{country_code, get_irs_country_data_entries},
        ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    TransferKind,
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
/// * refer to [`get_irs_country_data_entries`] errors.
pub fn can_receive(e: &Env, account: &Address, token: &Address) -> bool {
    let entries = get_irs_country_data_entries(e, token, account);
    for entry in entries.iter() {
        if is_country_allowed(e, token, country_code(&entry.country)) {
            return true;
        }
    }
    false
}

/// Rejects a transfer whose recipient has no allowed country, by panicking.
///
/// Country allowlist checks are recipient-based, so the sender and amount
/// are intentionally ignored. Forced (admin/recovery) transfers are exempt
/// from the policy, and no bookkeeping exists in this module, so they pass
/// through untouched.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient address.
/// * `kind` - Who initiated the transfer and under what authority.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::CountryNotAllowed`] - When the recipient has no
///   allowed country and the transfer is not forced.
/// * refer to [`can_receive`] errors.
pub fn on_transfer(e: &Env, to: &Address, kind: &TransferKind, token: &Address) {
    if *kind == TransferKind::Forced {
        return;
    }
    if !can_receive(e, to, token) {
        panic_with_error!(e, ComplianceModuleError::CountryNotAllowed);
    }
}

/// Rejects a mint whose recipient has no allowed country, by panicking.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient address.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::CountryNotAllowed`] - When the recipient has no
///   allowed country.
/// * refer to [`can_receive`] errors.
pub fn on_created(e: &Env, to: &Address, token: &Address) {
    if !can_receive(e, to, token) {
        panic_with_error!(e, ComplianceModuleError::CountryNotAllowed);
    }
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

/// Adds a country to the allowlist for `token`. If `country` is already
/// allowed, the call is a no-op (no event emitted, no error raised).
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

/// Removes a country from the allowlist for `token`. If `country` is not
/// currently allowed, the call is a no-op (no event emitted, no error
/// raised).
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

/// Adds multiple countries to the allowlist in a single call. Entries that
/// are already allowed are silently skipped (no event emitted, no error
/// raised).
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
///
/// Each `(token, country)` pair lives in its own persistent entry, so the
/// caller must size `countries` to stay within the per-transaction network
/// limits — see <https://lab.stellar.org/network-limits>.
pub fn batch_allow_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        add_allowed_country(e, token, country);
    }
}

/// Removes multiple countries from the allowlist in a single call. Entries
/// that are not currently allowed are silently skipped (no event emitted, no
/// error raised).
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
///
/// Each `(token, country)` pair lives in its own persistent entry, so the
/// caller must size `countries` to stay within the per-transaction network
/// limits — see <https://lab.stellar.org/network-limits>.
pub fn batch_disallow_countries(e: &Env, token: &Address, countries: &Vec<u32>) {
    for country in countries.iter() {
        remove_allowed_country(e, token, country);
    }
}
