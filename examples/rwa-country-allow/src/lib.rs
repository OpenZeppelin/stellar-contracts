#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::modules::{
    country_allow::{
        storage::{is_country_allowed, remove_country_allowed, set_country_allowed},
        CountryAllow, CountryAllowed, CountryUnallowed,
    },
    storage::{set_compliance_address, set_irs_address, ComplianceModuleStorageKey},
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct CountryAllowContract;

fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).expect("admin must be set")
}

fn require_module_admin_or_compliance_auth(e: &Env) {
    if let Some(compliance) =
        e.storage().instance().get::<_, Address>(&ComplianceModuleStorageKey::Compliance)
    {
        compliance.require_auth();
    } else {
        get_admin(e).require_auth();
    }
}

#[contractimpl]
impl CountryAllowContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl CountryAllow for CountryAllowContract {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn add_allowed_country(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        set_country_allowed(e, &token, country);
        CountryAllowed { token, country }.publish(e);
    }

    fn remove_allowed_country(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        remove_country_allowed(e, &token, country);
        CountryUnallowed { token, country }.publish(e);
    }

    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        for country in countries.iter() {
            set_country_allowed(e, &token, country);
            CountryAllowed { token: token.clone(), country }.publish(e);
        }
    }

    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        for country in countries.iter() {
            remove_country_allowed(e, &token, country);
            CountryUnallowed { token: token.clone(), country }.publish(e);
        }
    }

    fn is_country_allowed(e: &Env, token: Address, country: u32) -> bool {
        is_country_allowed(e, &token, country)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
