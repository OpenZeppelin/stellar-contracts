use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::modules::{
    country_allow::{storage as country_allow, CountryAllow},
    storage::{self as compliance_storage, set_irs_address},
    ComplianceModule,
};

#[contract]
pub struct CountryAllowContract;

#[contractimpl]
impl CountryAllowContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for CountryAllowContract {}

#[contractimpl(contracttrait)]
impl CountryAllow for CountryAllowContract {
    #[only_admin]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_admin]
    fn add_allowed_country(e: &Env, token: Address, country: u32) {
        country_allow::add_allowed_country(e, &token, country);
    }

    #[only_admin]
    fn remove_allowed_country(e: &Env, token: Address, country: u32) {
        country_allow::remove_allowed_country(e, &token, country);
    }

    #[only_admin]
    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        country_allow::batch_allow_countries(e, &token, &countries);
    }

    #[only_admin]
    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        country_allow::batch_disallow_countries(e, &token, &countries);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for CountryAllowContract {
    // Country modules have no state-mutating logic in these hooks; the
    // compliance check is performed by can_transfer / can_create.
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
        String::from_str(e, "CountryAllowModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
