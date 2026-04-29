use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::modules::{
    country_allow::{storage as country_allow, CountryAllow},
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
        country_allow::add_allowed_country(e, &token, country);
    }

    fn remove_allowed_country(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        country_allow::remove_allowed_country(e, &token, country);
    }

    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        country_allow::batch_allow_countries(e, &token, &countries);
    }

    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        country_allow::batch_disallow_countries(e, &token, &countries);
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
