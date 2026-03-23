#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::modules::{
    country_restrict::{
        storage::{is_country_restricted, remove_country_restricted, set_country_restricted},
        CountryRestrict, CountryRestricted, CountryUnrestricted,
    },
    storage::{set_compliance_address, set_irs_address, ComplianceModuleStorageKey},
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct CountryRestrictContract;

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
impl CountryRestrictContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl CountryRestrict for CountryRestrictContract {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn add_country_restriction(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        set_country_restricted(e, &token, country);
        CountryRestricted { token, country }.publish(e);
    }

    fn remove_country_restriction(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        remove_country_restricted(e, &token, country);
        CountryUnrestricted { token, country }.publish(e);
    }

    fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        for country in countries.iter() {
            set_country_restricted(e, &token, country);
            CountryRestricted { token: token.clone(), country }.publish(e);
        }
    }

    fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        for country in countries.iter() {
            remove_country_restricted(e, &token, country);
            CountryUnrestricted { token: token.clone(), country }.publish(e);
        }
    }

    fn is_country_restricted(e: &Env, token: Address, country: u32) -> bool {
        is_country_restricted(e, &token, country)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
