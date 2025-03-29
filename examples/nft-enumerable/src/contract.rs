//! Capped Example Contract.
//!
//! Demonstrates an example usage of `capped` module by
//! implementing a capped mint mechanism, and setting the maximum supply
//! at the constructor.
//!
//! **IMPORTANT**: this example is for demonstration purposes, and authorization
//! is not taken into consideration

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

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Enumerable;

    fn balance(e: &Env, owner: Address) -> Balance {
        Enumerable::balance(e, owner)
    }

    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        Enumerable::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        Enumerable::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        Enumerable::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        Enumerable::approve(e, approver, approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Enumerable::approve_for_all(e, owner, operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> {
        Enumerable::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Enumerable::is_approved_for_all(e, owner, operator)
    }

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

#[contractimpl]
impl NonFungibleEnumerable for ExampleContract {
    fn total_supply(e: &Env) -> Balance {
        Enumerable::total_supply(e)
    }

    fn get_owner_token_id(e: &Env, owner: Address, index: TokenId) -> TokenId {
        Enumerable::get_owner_token_id(e, &owner, index)
    }

    fn get_token_id(e: &Env, index: TokenId) -> TokenId {
        Enumerable::get_token_id(e, index)
    }
}

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
// impl NonFungibleSequentialMintable for ExampleContract {
//     fn mint(e: &Env, to: Address) -> TokenId {
//         non_fungible::mintable::sequential_mint(e, &to)
//     }
// }
//
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
