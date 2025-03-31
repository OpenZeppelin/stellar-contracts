//! Capped Example Contract.
//!
//! Demonstrates an example usage of `capped` module by
//! implementing a capped mint mechanism, and setting the maximum supply
//! at the constructor.
//!
//! **IMPORTANT**: this example is for demonstration purposes, and authorization
//! is not taken into consideration

use oz_stellar_macro::oz_stellar;
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_non_fungible::{
    burnable::NonFungibleBurnable, Balance, Base, NonFungibleToken, TokenId,
};

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

    pub fn mint(e: &Env, to: Address) -> TokenId {
        Base::sequential_mint(e, &to)
    }
}

#[oz_stellar]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;
}

#[oz_stellar]
#[contractimpl]
impl NonFungibleBurnable for ExampleContract {}
