//! Non-Fungible Vanilla Example Contract.
//!
//! Demonstrates an example usage of the NFT default base implementation.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_macros::{default_impl, only_owner};
use stellar_tokens::non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        set_owner(e, &owner);
        Base::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    #[only_owner]
    pub fn mint(e: &Env, to: Address) -> u32 {
        Base::sequential_mint(e, &to)
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

#[default_impl]
#[contractimpl]
impl Ownable for ExampleContract {}
