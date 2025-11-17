//! Non-Fungible Vanilla Example Contract.
//!
//! Demonstrates an example usage of the NFT default base implementation.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, uri: String, name: String, symbol: String, owner: Address) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        Base::set_metadata(e, uri, name, symbol);
    }

    pub fn mint(e: &Env, to: Address) -> u32 {
        let owner: Address =
            e.storage().instance().get(&DataKey::Owner).expect("owner should be set");
        owner.require_auth();
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
