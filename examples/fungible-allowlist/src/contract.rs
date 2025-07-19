//! Fungible AllowList Example Contract.

//! This contract showcases how to integrate the AllowList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can allow or disallow specific
//! accounts.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::fungible::{
    allowlist::{AllowList, FungibleAllowList},
    burnable::FungibleBurnable,
    Base, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, initial_supply: i128) {
        Base::set_metadata(
            e,
            18,
            String::from_str(e, "AllowList Token"),
            String::from_str(e, "ALT"),
        );

        access_control::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &admin, &manager, &symbol_short!("manager"));

        // Allow the admin to transfer tokens
        AllowList::allow_user(e, &admin);

        // Mint initial supply to the admin
        Base::mint(e, &admin, initial_supply);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = AllowList;
}
#[contractimpl]
impl FungibleAllowList for ExampleContract {
    fn allowed(e: &Env, account: Address) -> bool {
        AllowList::allowed(e, &account)
    }

    #[only_role(operator, "manager")]
    fn allow_user(e: &Env, user: Address, operator: Address) {
        AllowList::allow_user(e, &user)
    }

    #[only_role(operator, "manager")]
    fn disallow_user(e: &Env, user: Address, operator: Address) {
        AllowList::disallow_user(e, &user)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}

#[default_impl]
#[contractimpl]
impl FungibleBurnable for ExampleContract {}
