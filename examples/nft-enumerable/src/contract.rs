//! Non-Fungible Enumerable Example Contract.
//!
//! Demonstrates an example usage of the Enumerable extension, allowing for
//! enumeration of all the token IDs in the contract as well as all the token
//! IDs owned by each account.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::non_fungible::{
    burnable::NonFungibleBurnable,
    enumerable::{Enumerable, NonFungibleEnumerable},
    Base, NonFungibleToken,
};

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        Base::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    pub fn mint(e: &Env, to: Address) -> u32 {
        let owner: Address =
            e.storage().instance().get(&DataKey::Owner).expect("owner should be set");
        owner.require_auth();
        Enumerable::sequential_mint(e, &to)
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Enumerable;
}

#[default_impl]
#[contractimpl]
impl NonFungibleEnumerable for ExampleContract {}

#[default_impl]
#[contractimpl]
impl NonFungibleBurnable for ExampleContract {}
