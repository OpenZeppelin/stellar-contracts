//! Non-Fungible Vanilla Example Contract.
//!
//! Demonstrates an example usage of the NFT default base implementation.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

use stellar_non_fungible::{NonFungibleBurnable, NonFungibleToken};
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
        Self::enforce_owner_auth(e);
        Self::sequential_mint(e, &to)
    }
}

#[contracttrait]
impl NonFungibleToken for ExampleContract {}

#[contracttrait]
impl Ownable for ExampleContract {}

#[contracttrait]
impl NonFungibleBurnable for ExampleContract {}
