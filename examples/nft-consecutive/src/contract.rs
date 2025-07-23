//! Non-Fungible Consecutive Example Contract.
//!
//! Demonstrates an example usage of the Consecutive extension, enabling
//! efficient batch minting in a single transaction.

use soroban_sdk::{contract, contractimpl, contracttrait, Address, Env, String};
use stellar_non_fungible::{consecutive::Consecutive, NonFungibleBurnable, NonFungibleToken};
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

    pub fn batch_mint(e: &Env, to: Address, amount: u32) -> u32 {
        Self::only_owner(e);
        Consecutive::batch_mint(e, &to, amount)
    }
}

#[contracttrait(default = Consecutive)]
impl NonFungibleToken for ExampleContract {}

#[contracttrait(default = Consecutive)]
impl NonFungibleBurnable for ExampleContract {}

#[contracttrait]
impl Ownable for ExampleContract {}
