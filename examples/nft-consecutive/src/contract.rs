//! Non-Fungible Consecutive Example Contract.
//!
//! Demonstrates an example usage of the Consecutive extension, enabling
//! efficient batch minting in a single transaction.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_macros::{default_impl, only_owner};
use stellar_tokens::non_fungible::{
    burnable::NonFungibleBurnable,
    consecutive::{Consecutive, NonFungibleConsecutive},
    Base, ContractOverrides, NonFungibleToken,
};

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
    pub fn batch_mint(e: &Env, to: Address, amount: u32) -> u32 {
        Consecutive::batch_mint(e, &to, amount)
    }
}

// You don't have to provide the implementations for all the methods,
// `#[default_impl]` macro does this for you. This example showcases
// what is happening under the hood when you use `#[default_impl]` macro.
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Consecutive;

    fn balance(e: &Env, owner: Address) -> u32 {
        Self::ContractType::balance(e, &owner)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        Self::ContractType::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        Self::ContractType::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        Self::ContractType::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Self::ContractType::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Self::ContractType::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
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

    fn token_uri(e: &Env, token_id: u32) -> String {
        Self::ContractType::token_uri(e, token_id)
    }
}

impl NonFungibleConsecutive for ExampleContract {}

#[contractimpl]
impl NonFungibleBurnable for ExampleContract {
    fn burn(e: &Env, from: Address, token_id: u32) {
        Self::ContractType::burn(e, &from, token_id);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        Self::ContractType::burn_from(e, &spender, &from, token_id);
    }
}

#[default_impl]
#[contractimpl]
impl Ownable for ExampleContract {}
