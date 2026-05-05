use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use stellar_access::access_control;
use stellar_tokens::rwa::compliance::modules::{
    country_allow::{storage as country_allow, CountryAllow},
    storage::{
        get_compliance_address, module_name, set_compliance_address, set_irs_address,
        ComplianceModuleStorageKey,
    },
    ComplianceModule,
};

#[contract]
pub struct CountryAllowContract;

fn get_admin(e: &Env) -> Address {
    access_control::get_admin(e).expect("admin is set in the constructor")
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
        access_control::set_admin(e, &admin);
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
}

#[contractimpl(contracttrait)]
impl ComplianceModule for CountryAllowContract {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        country_allow::can_transfer(e, &from, &to, amount, &token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        country_allow::can_create(e, &to, amount, &token)
    }

    fn name(e: &Env) -> String {
        module_name(e, "CountryAllowModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
