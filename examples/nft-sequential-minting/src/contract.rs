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

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;

    fn balance(e: &Env, owner: Address) -> Balance {
        Self::ContractType::balance(e, &owner)
    }

    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        Self::ContractType::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        Self::ContractType::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        Self::ContractType::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        Self::ContractType::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Self::ContractType::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> {
        Self::ContractType::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Self::ContractType::is_approved_for_all(e, &owner, &operator)
    }

    fn name(e: &Env) -> String {
        Self::ContractType::name(e)
    }

    fn symbol(e: &Env) -> String {
        Self::ContractType::symbol(e)
    }

    fn token_uri(e: &Env, token_id: TokenId) -> String {
        Self::ContractType::token_uri(e, token_id)
    }
}

#[contractimpl]
impl NonFungibleBurnable for ExampleContract {
    fn burn(e: &Env, from: Address, token_id: TokenId) {
        Base::burn(e, &from, token_id);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: TokenId) {
        Base::burn_from(e, &spender, &from, token_id);
    }
}
