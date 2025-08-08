//! Non-Fungible Consecutive Example Contract.
//!
//! Demonstrates an example usage of the Consecutive extension, enabling
//! efficient batch minting in a single transaction.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_access::{Ownable, Owner};
use stellar_tokens::{
    non_fungible::consecutive::Consecutive, NonFungibleBurnable, NonFungibleToken,
};

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

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type Impl = Consecutive;
}

#[contractimpl]
impl NonFungibleBurnable for ExampleContract {
    type Impl = Consecutive;
}

#[contractimpl]
impl Ownable for ExampleContract {
    type Impl = Owner;
}
