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
    enumerable::{Enumerable, NonFungibleEnumerable},
    Balance, Base, ContractOverrides, NonFungibleToken, TokenId,
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

#[oz_stellar]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Enumerable;

    fn name(e: &Env) -> String {
        Enumerable::name(e)
    }

    fn symbol(e: &Env) -> String {
        Enumerable::symbol(e)
    }

    fn token_uri(e: &Env, token_id: TokenId) -> String {
        Enumerable::token_uri(e, token_id)
    }
}

#[oz_stellar]
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
