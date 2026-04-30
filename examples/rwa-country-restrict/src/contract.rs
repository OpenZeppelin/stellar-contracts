use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::modules::{
    country_restrict::storage as country_restrict,
    storage::{
        get_compliance_address, module_name, set_compliance_address, set_irs_address,
        ComplianceModuleStorageKey,
    },
    ComplianceModule,
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

    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    pub fn add_country_restriction(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        country_restrict::add_country_restriction(e, &token, country);
    }

    pub fn remove_country_restriction(e: &Env, token: Address, country: u32) {
        require_module_admin_or_compliance_auth(e);
        country_restrict::remove_country_restriction(e, &token, country);
    }

    pub fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        country_restrict::batch_restrict_countries(e, &token, &countries);
    }

    pub fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_module_admin_or_compliance_auth(e);
        country_restrict::batch_unrestrict_countries(e, &token, &countries);
    }

    pub fn is_country_restricted(e: &Env, token: Address, country: u32) -> bool {
        country_restrict::is_country_restricted(e, &token, country)
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for CountryRestrictContract {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(e: &Env, _from: Address, to: Address, _amount: i128, token: Address) -> bool {
        country_restrict::can_transfer(e, &to, &token)
    }

    fn can_create(e: &Env, to: Address, _amount: i128, token: Address) -> bool {
        country_restrict::can_transfer(e, &to, &token)
    }

    fn name(e: &Env) -> String {
        module_name(e, "CountryRestrictModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
