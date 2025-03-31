//! Non-Fungible Enumerable Example Contract.
//!
//! Demonstrates an example usage of the Enumerable extension, allowing for
//! enumeration of all the token IDs in the contract as well as all the token
//! IDs owned by each account.
//!
//! **IMPORTANT**: This example is for demonstration purposes, and access
//! control to sensitive operations is not taken into consideration!

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_default_impl_macro::default_impl;
use stellar_non_fungible::{
    enumerable::{Enumerable, NonFungibleEnumerable},
    Balance, Base, NonFungibleToken, TokenId,
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
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Enumerable;
}

#[default_impl]
#[contractimpl]
impl NonFungibleEnumerable for ExampleContract {}

#[contractimpl]
impl ExampleContract {
    pub fn mint(e: &Env, to: Address) -> TokenId {
        Enumerable::sequential_mint(e, &to)
    }

    pub fn burn(e: &Env, from: Address, token_id: TokenId) {
        Enumerable::sequential_burn(e, &from, token_id);
    }
}

/*
  BELOW WILL CREATE A COMPILE ERROR,
  SINCE ENUMERABLE IS NOT COMPATIBLE WITH THEM
*/

// ```rust
// #[contractimpl]
// impl NonFungibleBurnable for ExampleContract {
//     fn burn(e: &Env, from: Address, token_id: TokenId) {
//         Base::burn(e, &from, token_id);
//     }
//
//     fn burn_from(e: &Env, spender: Address, from: Address, token_id: TokenId) {
//         Base::burn_from(e, &spender, &from, token_id);
//     }
// }
// ```
