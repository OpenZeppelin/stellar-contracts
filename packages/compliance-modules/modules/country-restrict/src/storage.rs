use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum CountryRestrictStorageKey {
    /// Per-(token, country) restriction flag.
    RestrictedCountry(Address, u32),
}

pub fn is_country_restricted(e: &Env, token: &Address, country: u32) -> bool {
    e.storage()
        .persistent()
        .get(&CountryRestrictStorageKey::RestrictedCountry(token.clone(), country))
        .unwrap_or_default()
}

pub fn set_country_restricted(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .set(&CountryRestrictStorageKey::RestrictedCountry(token.clone(), country), &true);
}

pub fn remove_country_restricted(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .remove(&CountryRestrictStorageKey::RestrictedCountry(token.clone(), country));
}
