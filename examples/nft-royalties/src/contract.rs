//! Non-Fungible Royalties Example Contract.
//!
//! Demonstrates an example usage of the Royalties extension, allowing for
//! setting and querying royalty information for NFTs following the ERC2981
//! standard.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};
use stellar_default_impl_macro::default_impl;
use stellar_non_fungible::{royalties::NonFungibleRoyalties, Base, NonFungibleToken};

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        Base::set_metadata(
            e,
            String::from_str(e, "https://example.com/nft/"),
            String::from_str(e, "Royalty NFT"),
            String::from_str(e, "RNFT"),
        );

        // Set default royalty for the entire collection (10%)
        Base::set_default_royalty(e, &owner, 1000);
    }

    pub fn mint(e: &Env, to: Address) -> u32 {
        let owner: Address =
            e.storage().instance().get(&DataKey::Owner).expect("owner should be set");
        owner.require_auth();

        // Mint token with sequential ID
        Base::sequential_mint(e, &to)
    }

    pub fn mint_with_royalty(e: &Env, to: Address, receiver: Address, basis_points: u32) -> u32 {
        let owner: Address =
            e.storage().instance().get(&DataKey::Owner).expect("owner should be set");
        owner.require_auth();

        // Mint token with sequential ID
        let token_id = Base::sequential_mint(e, &to);

        // Set token-specific royalty
        Base::set_token_royalty(e, token_id, &receiver, basis_points);

        token_id
    }

    pub fn get_royalty_info(e: &Env, token_id: u32, sale_price: u32) -> (Address, u32) {
        Base::royalty_info(e, token_id, sale_price)
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;
}

#[default_impl]
#[contractimpl]
impl NonFungibleRoyalties for ExampleContract {}
