use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::fungible::{self as fungible, sac_admin_wrapper::SACAdminWrapper};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, default_admin: Address, manager: Address, sac: Address) {
        access_control::set_admin(e, &default_admin);

        // create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &manager, &symbol_short!("manager"), &default_admin);

        fungible::sac_admin_wrapper::set_sac_address(e, &sac);
    }
}

#[contractimpl]
impl SACAdminWrapper for ExampleContract {
    #[only_admin]
    fn set_admin(e: Env, new_admin: Address, _operator: Address) {
        fungible::sac_admin_wrapper::set_admin(&e, &new_admin);
    }

    #[only_role(operator, "manager")]
    fn set_authorized(e: Env, id: Address, authorize: bool, operator: Address) {
        fungible::sac_admin_wrapper::set_authorized(&e, &id, authorize);
    }

    #[only_role(operator, "manager")]
    fn mint(e: Env, to: Address, amount: i128, operator: Address) {
        fungible::sac_admin_wrapper::mint(&e, &to, amount);
    }

    #[only_role(operator, "manager")]
    fn clawback(e: Env, from: Address, amount: i128, operator: Address) {
        fungible::sac_admin_wrapper::clawback(&e, &from, amount);
    }
}

#[contractimpl(contracttrait = true)]
impl AccessControl for ExampleContract {}
