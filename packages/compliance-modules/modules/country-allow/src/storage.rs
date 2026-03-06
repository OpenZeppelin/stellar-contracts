use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum CountryAllowStorageKey {
    /// Per-(token, country) allowlist flag.
    AllowedCountry(Address, u32),
}

pub fn is_country_allowed(e: &Env, token: &Address, country: u32) -> bool {
    e.storage()
        .persistent()
        .get(&CountryAllowStorageKey::AllowedCountry(token.clone(), country))
        .unwrap_or_default()
}

pub fn set_country_allowed(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .set(&CountryAllowStorageKey::AllowedCountry(token.clone(), country), &true);
}

pub fn remove_country_allowed(e: &Env, token: &Address, country: u32) {
    e.storage()
        .persistent()
        .remove(&CountryAllowStorageKey::AllowedCountry(token.clone(), country));
}
