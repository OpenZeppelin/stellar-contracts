// TODO: Refactor to use access_control and/or ownable when merged
use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, vec, Address, Env,
    IntoVal, Symbol, Val, Vec,
};
use stellar_fungible::{self as fungible, sac_admin_wrapper::SACAdminWrapper};

pub const OWNER: Symbol = symbol_short!("OWNER");

#[contract]
pub struct ExampleSACAdminWrapperContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleSACAdminWrapperContractError {
    Unauthorized = 1,
}

#[contractimpl]
impl ExampleSACAdminWrapperContract {
    pub fn __constructor(e: &Env, owner: Address, sac: Address) {
        fungible::sac_admin_wrapper::set_sac_address(e, &sac);
        e.storage().instance().set(&OWNER, &owner);
    }
}

#[contractimpl]
impl SACAdminWrapper for ExampleSACAdminWrapperContract {
    fn set_admin(e: Env, new_admin: Address, operator: Address) {
        auth_owner(&e, &operator, vec![&e, new_admin.into_val(&e)]);
        fungible::sac_admin_wrapper::set_admin(&e, &new_admin);
    }

    fn set_authorized(e: Env, id: Address, authorize: bool, operator: Address) {
        auth_owner(&e, &operator, vec![&e, id.into_val(&e), authorize.into_val(&e)]);
        fungible::sac_admin_wrapper::set_authorized(&e, &id, authorize);
    }

    fn mint(e: Env, to: Address, amount: i128, operator: Address) {
        auth_owner(&e, &operator, vec![&e, to.into_val(&e), amount.into_val(&e)]);
        fungible::sac_admin_wrapper::mint(&e, &to, amount);
    }

    fn clawback(e: Env, from: Address, amount: i128, operator: Address) {
        auth_owner(&e, &operator, vec![&e, from.into_val(&e), amount.into_val(&e)]);
        fungible::sac_admin_wrapper::clawback(&e, &from, amount);
    }
}

fn auth_owner(e: &Env, operator: &Address, args: Vec<Val>) {
    operator.require_auth_for_args(args);
    let owner: Address = e.storage().instance().get(&OWNER).expect("owner must be set");
    if *operator != owner {
        panic_with_error!(e, ExampleSACAdminWrapperContractError::Unauthorized)
    }
}
