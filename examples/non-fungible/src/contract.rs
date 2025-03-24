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
    self as non_fungible, enumerable::NonFungibleEnumerable, ContractBehavior, EnumerableContract,
    NonFungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn mint(e: &Env, to: Address) {
        non_fungible::enumerable::storage::sequential_mint(e, &to);
    }
}

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = EnumerableContract;

    fn balance(e: &Env, owner: Address) -> u32 {
        non_fungible::balance(e, &owner)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        Self::ContractType::transfer(e, from, to, token_id);
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        non_fungible::owner_of(e, token_id)
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        non_fungible::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        non_fungible::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        non_fungible::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        non_fungible::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        non_fungible::is_approved_for_all(e, &owner, &operator)
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "LOL")
    }

    fn symbol(e: &Env) -> String {
        String::from_str(e, "LOL")
    }

    fn token_uri(e: &Env, _token_id: u32) -> String {
        String::from_str(e, "LOL")
    }
}

#[contractimpl]
impl NonFungibleEnumerable for ExampleContract {
    fn total_supply(e: &Env) -> u32 {
        non_fungible::enumerable::storage::total_supply(e)
    }

    fn get_owner_token_id(e: &Env, owner: Address, index: u32) -> u32 {
        non_fungible::enumerable::storage::get_owner_token_id(e, &owner, index)
    }

    fn get_token_id(e: &Env, index: u32) -> u32 {
        non_fungible::enumerable::storage::get_token_id(e, index)
    }
}
