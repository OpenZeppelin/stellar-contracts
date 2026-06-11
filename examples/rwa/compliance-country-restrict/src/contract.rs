use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::{
    modules::{
        country_restrict::{storage as country_restrict, CountryRestrict},
        storage::{self as compliance_storage, set_irs_address},
        ComplianceModule,
    },
    AccountSnapshot,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct CountryRestrictContract;

#[contractimpl]
impl CountryRestrictContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for CountryRestrictContract {}

#[contractimpl(contracttrait)]
impl CountryRestrict for CountryRestrictContract {
    #[only_role(operator, "manager")]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address, operator: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_role(operator, "manager")]
    fn add_country_restriction(e: &Env, token: Address, country: u32, operator: Address) {
        country_restrict::add_country_restriction(e, &token, country);
    }

    #[only_role(operator, "manager")]
    fn remove_country_restriction(e: &Env, token: Address, country: u32, operator: Address) {
        country_restrict::remove_country_restriction(e, &token, country);
    }

    #[only_role(operator, "manager")]
    fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address) {
        country_restrict::batch_restrict_countries(e, &token, &countries);
    }

    #[only_role(operator, "manager")]
    fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address) {
        country_restrict::batch_unrestrict_countries(e, &token, &countries);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for CountryRestrictContract {
    // No need to implement logic in these hooks for this module, as the compliance
    // check is only done in the can_transfer and can_create functions.
    fn on_transfer(
        _e: &Env,
        _from: AccountSnapshot,
        _to: AccountSnapshot,
        _amount: i128,
        _spender: Option<Address>,
        _token: Address,
    ) {
    }

    // No need to implement logic in these hooks for this module, as the compliance
    // check is only done in the can_transfer and can_create functions.
    fn on_created(_e: &Env, _to: AccountSnapshot, _amount: i128, _token: Address) {}

    // No need to implement logic in these hooks for this module, as the compliance
    // check is only done in the can_transfer and can_create functions.
    fn on_destroyed(_e: &Env, _from: AccountSnapshot, _amount: i128, _token: Address) {}

    fn can_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        _spender: Option<Address>,
        token: Address,
    ) -> bool {
        country_restrict::can_transfer(e, &from.address, &to.address, amount, &token)
    }

    fn can_create(e: &Env, to: AccountSnapshot, amount: i128, token: Address) -> bool {
        country_restrict::can_create(e, &to.address, amount, &token)
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "CountryRestrictModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
