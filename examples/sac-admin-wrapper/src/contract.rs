// TODO: Refactor to use access_control and/or ownable when merged
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};
use stellar_access_control::{self as access_control, AccessControl};
use stellar_access_control_macro::has_role;
use stellar_default_impl_macro::default_impl;
use stellar_fungible::{self as fungible, sac_admin_wrapper::SACAdminWrapper};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, default_admin: Address, manager: Address, sac: Address) {
        access_control::set_admin(e, &default_admin);

        // create a role "chief" and grant it to `default_admin`
        access_control::grant_role(e, &default_admin, &default_admin, &symbol_short!("chief"));

        // create a role "manager" and grant it to `manager`
        access_control::grant_role(e, &default_admin, &manager, &symbol_short!("manager"));

        fungible::sac_admin_wrapper::set_sac_address(e, &sac);
    }
}

#[contractimpl]
impl SACAdminWrapper for ExampleContract {
    #[has_role(operator, "chief")]
    fn set_admin(e: Env, new_admin: Address, operator: Address) {
        fungible::sac_admin_wrapper::set_admin(&e, &new_admin);
    }

    #[has_role(operator, "manager")]
    fn set_authorized(e: Env, id: Address, authorize: bool, operator: Address) {
        fungible::sac_admin_wrapper::set_authorized(&e, &id, authorize);
    }

    #[has_role(operator, "manager")]
    fn mint(e: Env, to: Address, amount: i128, operator: Address) {
        fungible::sac_admin_wrapper::mint(&e, &to, amount);
    }

    #[has_role(operator, "chief")]
    fn clawback(e: Env, from: Address, amount: i128, operator: Address) {
        fungible::sac_admin_wrapper::clawback(&e, &from, amount);
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}
