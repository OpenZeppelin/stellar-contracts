use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::modules::{
    country_restrict::{storage as country_restrict, CountryRestrict},
    storage::{self as compliance_storage, set_irs_address},
    ComplianceModule,
};

#[contract]
pub struct CountryRestrictContract;

#[contractimpl]
impl CountryRestrictContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for CountryRestrictContract {}

#[contractimpl(contracttrait)]
impl CountryRestrict for CountryRestrictContract {
    #[only_admin]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_admin]
    fn add_country_restriction(e: &Env, token: Address, country: u32) {
        country_restrict::add_country_restriction(e, &token, country);
    }

    #[only_admin]
    fn remove_country_restriction(e: &Env, token: Address, country: u32) {
        country_restrict::remove_country_restriction(e, &token, country);
    }

    #[only_admin]
    fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        country_restrict::batch_restrict_countries(e, &token, &countries);
    }

    #[only_admin]
    fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        country_restrict::batch_unrestrict_countries(e, &token, &countries);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for CountryRestrictContract {
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
        country_restrict::can_transfer(e, &from, &to, amount, &token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        country_restrict::can_create(e, &to, amount, &token)
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "Country Restriction Module")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
