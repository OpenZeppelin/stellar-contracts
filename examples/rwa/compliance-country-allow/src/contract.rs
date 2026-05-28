use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::modules::{
    country_allow::{storage as country_allow, CountryAllow},
    storage::{self as compliance_storage, set_irs_address},
    ComplianceModule,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct CountryAllowContract;

#[contractimpl]
impl CountryAllowContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for CountryAllowContract {}

#[contractimpl(contracttrait)]
impl CountryAllow for CountryAllowContract {
    #[only_role(operator, "manager")]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address, operator: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_role(operator, "manager")]
    fn add_allowed_country(e: &Env, token: Address, country: u32, operator: Address) {
        country_allow::add_allowed_country(e, &token, country);
    }

    #[only_role(operator, "manager")]
    fn remove_allowed_country(e: &Env, token: Address, country: u32, operator: Address) {
        country_allow::remove_allowed_country(e, &token, country);
    }

    #[only_role(operator, "manager")]
    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address) {
        country_allow::batch_allow_countries(e, &token, &countries);
    }

    #[only_role(operator, "manager")]
    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address) {
        country_allow::batch_disallow_countries(e, &token, &countries);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for CountryAllowContract {
    // No need to implement logic in these hooks for this module, as the compliance
    // check is only done in the can_transfer and can_create functions.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    // No need to implement logic in these hooks for this module, as the compliance
    // check is only done in the can_transfer and can_create functions.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    // No need to implement logic in these hooks for this module, as the compliance
    // check is only done in the can_transfer and can_create functions.
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
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
