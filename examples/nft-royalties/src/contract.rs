//! Non-Fungible Royalties Example Contract.
//!
//! Demonstrates an example usage of the Royalties extension, allowing for
//! setting and querying royalty information for NFTs following the ERC2981
//! standard.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol};
use stellar_access::{AccessControl, AccessController};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::non_fungible::{royalties::NonFungibleRoyalties, NFTBase, NonFungibleToken};

const MANAGER: &Symbol = &symbol_short!("manager");

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: &Address, manager: &Address) {
        Self::set_metadata(
            e,
            String::from_str(e, "https://example.com/nft/"),
            String::from_str(e, "Royalty NFT"),
            String::from_str(e, "RNFT"),
        );

        Self::init_admin(e, admin);

        // create a role "manager" and grant it to `manager`
        Self::grant_role_no_auth(e, admin, manager, MANAGER);
        Self::grant_role_no_auth(e, admin, admin, MANAGER);

        // Set default royalty for the entire collection (10%)
        Self::set_default_royalty(e, admin, 1000, admin);
    }

    #[only_admin]
    pub fn mint(e: &Env, to: Address) -> u32 {
        // Mint token with sequential ID
        Self::sequential_mint(e, &to)
    }

    // Don't need a check here since it is done in set_token_royalty
    pub fn mint_with_royalty(e: &Env, to: &Address, receiver: &Address, basis_points: u32) -> u32 {
        // Mint token with sequential ID
        let token_id = Self::sequential_mint(e, to);
        // Set token-specific royalty
        Self::set_token_royalty(e, token_id, receiver, basis_points, to);
        token_id
    }
}

#[contractimpl]
impl AccessControl for ExampleContract {
    type Impl = AccessController;
}

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type Impl = NFTBase;
}

#[contractimpl]
impl NonFungibleRoyalties for ExampleContract {
    type Impl = NFTBase;

    #[only_role(operator, "manager")]
    fn set_default_royalty(e: &Env, receiver: &Address, basis_points: u32, operator: &Address) {
        Self::Impl::set_default_royalty(e, receiver, basis_points, operator);
    }

    #[only_role(operator, "manager")]
    fn set_token_royalty(
        e: &Env,
        token_id: u32,
        receiver: &Address,
        basis_points: u32,
        operator: &Address,
    ) {
        Self::Impl::set_token_royalty(e, token_id, receiver, basis_points, operator);
    }

    #[only_role(operator, "manager")]
    fn remove_token_royalty(e: &Env, token_id: u32, operator: &Address) {
        Self::Impl::remove_token_royalty(e, token_id, operator);
    }

    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
        Self::Impl::royalty_info(e, token_id, sale_price)
    }
}
