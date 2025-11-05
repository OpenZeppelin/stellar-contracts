//! RWA Example Contract.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::default_impl;
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    rwa::RWA,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, initial_supply: i128) {
        Base::set_metadata(e, 18, String::from_str(e, "RWA Token"), String::from_str(e, "RWA"));

        access_control::set_admin(e, &admin);

        // Create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &admin, &manager, &symbol_short!("manager"));

        // Mint initial supply to the admin
        RWA::mint(e, &admin, initial_supply);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = RWA;
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}
