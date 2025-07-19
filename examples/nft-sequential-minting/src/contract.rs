//! Non-Fungible Vanilla Example Contract.
//!
//! Demonstrates an example usage of the NFT default base implementation.

use soroban_sdk::{contract, contractimpl, derive_contract, Address, Env, String};

use stellar_non_fungible::{NonFungibleBurnable, NonFungibleToken};
use stellar_ownable::Ownable;

#[contract]
#[derive_contract(NonFungibleToken, Ownable, NonFungibleBurnable)]
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
