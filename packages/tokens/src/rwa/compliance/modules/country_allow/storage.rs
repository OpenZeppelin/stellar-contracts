use soroban_sdk::{contracttype, Address, Env};

use crate::rwa::compliance::modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

#[contracttype]
#[derive(Clone)]
pub enum CountryAllowStorageKey {
    /// Per-(token, country) allowlist flag.
    AllowedCountry(Address, u32),
}

/// Returns whether the given country is on the allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code.
pub fn is_country_allowed(e: &Env, token: &Address, country: u32) -> bool {
    let key = CountryAllowStorageKey::AllowedCountry(token.clone(), country);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &bool| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Adds a country to the allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `country` - The ISO 3166-1 numeric country code to allow.
pub fn set_country_allowed(e: &Env, token: &Address, country: u32) {
    let key = CountryAllowStorageKey::AllowedCountry(token.clone(), country);
    e.storage().persistent().set(&key, &true);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Removes a country from the allowlist for `token`.
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
