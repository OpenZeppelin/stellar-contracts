//! Fungible AllowList Example Contract.

//! This contract showcases how to integrate the AllowList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can allow or disallow specific accounts.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_default_impl_macro::default_impl;
use stellar_fungible::{
    allowlist::{AllowList, FungibleAllowList},
    Base, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, initial_supply: i128) {
        Base::set_metadata(
            e,
            18,
            String::from_str(e, "AllowList Token"),
            String::from_str(e, "ALT"),
        );

        // Set the admin for the AllowList extension
        AllowList::set_admin(e, &admin);

        // Allow the admin to transfer tokens
        AllowList::allow_user_no_auth(e, &admin);

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

    fn allow_user(e: &Env, user: Address) {
        AllowList::allow_user(e, &user)
    }

    fn disallow_user(e: &Env, user: Address) {
        AllowList::disallow_user(e, &user)
    }
}
