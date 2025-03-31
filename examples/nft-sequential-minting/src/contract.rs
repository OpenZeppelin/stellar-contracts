//! Non-Fungible Vanilla Example Contract.
//!
//! Demonstrates an example usage of the NFT default base implementation.
//!
//! **IMPORTANT**: This example is for demonstration purposes, and access
//! control to sensitive operations is not taken into consideration!

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_default_impl_macro::default_impl;
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

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;
}

#[default_impl]
#[contractimpl]
impl NonFungibleBurnable for ExampleContract {}
