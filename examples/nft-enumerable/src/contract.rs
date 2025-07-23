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
use stellar_ownable::Ownable;

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Self::set_owner(e, &owner);
        Self::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    pub fn mint(e: &Env, to: Address) -> u32 {
        Self::only_owner(e);
        Enumerable::sequential_mint(e, &to)
    }
}

#[contracttrait(default = Enumerable)]
impl NonFungibleToken for ExampleContract {}

#[contracttrait(default = Enumerable)]
impl NonFungibleBurnable for ExampleContract {}

#[contracttrait]
impl Ownable for ExampleContract {}

#[contracttrait]
impl NonFungibleEnumerable for ExampleContract {}
