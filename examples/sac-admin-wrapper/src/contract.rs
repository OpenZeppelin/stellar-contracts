use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};
use stellar_access::{AccessControl, AccessController};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::fungible::sac_admin_wrapper::{DefaultSacAdminWrapper, SACAdminWrapper};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(
        e: &Env,
        default_admin: Address,
        manager1: Address,
        manager2: Address,
        sac: Address,
    ) {
        Self::init_admin(e, &default_admin);

        // create a role "manager" and grant it to `manager1`
        Self::grant_role_no_auth(e, &default_admin, &manager1, &symbol_short!("manager"));

        // grant it to `manager2`
        Self::grant_role_no_auth(e, &default_admin, &manager2, &symbol_short!("manager"));

        Self::set_sac_address(e, &sac);
    }
}

#[contractimpl]
impl AccessControl for ExampleContract {
    type Impl = AccessController;
}

#[contractimpl]
impl SACAdminWrapper for ExampleContract {
    type Impl = DefaultSacAdminWrapper;

    #[only_admin]
    fn set_admin(e: &Env, new_admin: &Address, _operator: &Address) {
        Self::Impl::set_admin(e, new_admin, _operator);
    }

    #[only_role(operator, "manager")]
    fn set_authorized(e: &Env, id: &Address, authorize: bool, operator: &Address) {
        Self::Impl::set_authorized(e, id, authorize, operator);
    }

    #[only_role(operator, "manager")]
    fn mint(e: &Env, to: &Address, amount: i128, operator: &Address) {
        Self::Impl::mint(e, to, amount, operator);
    }

    #[only_role(operator, "manager")]
    fn clawback(e: &Env, from: &Address, amount: i128, operator: &Address) {
        Self::Impl::clawback(e, from, amount, operator);
    }
}
