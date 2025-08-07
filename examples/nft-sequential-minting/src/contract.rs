//! Non-Fungible Vanilla Example Contract.
//!
//! Demonstrates an example usage of the NFT default base implementation.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_access::{Ownable, Owner};
use stellar_tokens::{NFTBase, NonFungibleBurnable, NonFungibleToken};

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

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type Impl = NFTBase;
}

#[contractimpl]
impl Ownable for ExampleContract {
    type Impl = Owner;
}

#[contractimpl]
impl NonFungibleBurnable for ExampleContract {
    type Impl = NFTBase;
}
