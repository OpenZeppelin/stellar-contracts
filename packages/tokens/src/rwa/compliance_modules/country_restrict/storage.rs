use soroban_sdk::{contracttype, Address, Env};

use crate::rwa::compliance_modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

#[contracttype]
#[derive(Clone)]
pub enum CountryRestrictStorageKey {
    /// Per-(token, country) restriction flag.
    RestrictedCountry(Address, u32),
}

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

/// Adds a country to the restriction list for `token`.
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

/// Removes a country from the restriction list for `token`.
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
