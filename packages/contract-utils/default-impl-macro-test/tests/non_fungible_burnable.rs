//! Non-Fungible Enumerable Example Contract.
//!
//! Demonstrates an example usage of the Enumerable extension, allowing for
//! enumeration of all the token IDs in the contract as well as all the token
//! IDs owned by each account.
//!
//! **IMPORTANT**: This example is for demonstration purposes, and access
//! control to sensitive operations is not taken into consideration!

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_default_impl_macro::default_impl;
use stellar_non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env) {
        Base::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;
}

#[default_impl]
#[contractimpl]
impl NonFungibleBurnable for ExampleContract {}

#[contractimpl]
impl ExampleContract {
    pub fn mint(e: &Env, to: Address, token_id: u32) {
        Base::mint(e, &to, token_id);
    }
}

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_non_fungible_burnable_burn() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    client.burn(&owner, &10);
    assert_eq!(client.balance(&owner), 0);
}

#[test]
fn default_impl_non_fungible_burnable_burn_from() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    client.approve(&owner, &spender, &10, &1000);
    client.burn_from(&spender, &owner, &10);
    assert_eq!(client.balance(&owner), 0);
}
