//! Non-Fungible Royalties Example Contract.
//!
//! Demonstrates an example usage of the Royalties extension, allowing for
//! setting and querying royalty information for NFTs following the ERC2981
//! standard.

use soroban_sdk::{contract, contractimpl, derive_contract, symbol_short, Address, Env, String};
use stellar_access_control::AccessControl;
use stellar_access_control_macros::has_role;
use stellar_non_fungible::{NonFungibleRoyalties, NonFungibleToken};

#[contract]
#[derive_contract(
    NonFungibleToken,
    NonFungibleRoyalties(default = ExampleContract),
    AccessControl,
)]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        Self::set_metadata(
            e,
            String::from_str(e, "https://example.com/nft/"),
            String::from_str(e, "Royalty NFT"),
            String::from_str(e, "RNFT"),
        );

        // Set default royalty for the entire collection (10%)
        <NonFungibleRoyalties!()>::set_default_royalty(e, &admin, 1000, &admin);

        Self::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        <Self as AccessControl>::grant_role(e, &admin, &manager, &symbol_short!("manager"));
    }

    // #[only_admin]
    pub fn mint(e: &Env, to: Address) -> u32 {
        Self::enforce_admin_auth(e);
        // Mint token with sequential ID
        Self::sequential_mint(e, &to)
    }

    // #[only_admin]
    pub fn mint_with_royalty(e: &Env, to: Address, receiver: Address, basis_points: u32) -> u32 {
        // Mint token with sequential ID
        Self::enforce_admin_auth(e);
        let token_id = Self::sequential_mint(e, &to);

        // Set token-specific royalty
        <Self as NonFungibleRoyalties>::set_token_royalty(
            e,
            token_id,
            &receiver,
            basis_points,
            &receiver,
        );

        token_id
    }

    // pub fn get_royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
    //     <Self::royalty_info(e, token_id, sale_price)
    // }
}

impl NonFungibleRoyalties for ExampleContract {
    type Impl = NonFungibleRoyalties!();
    #[has_role(operator, "manager")]
    fn set_default_royalty(e: &Env, receiver: &Address, basis_points: u32, operator: &Address) {
        Self::Impl::set_default_royalty(e, receiver, basis_points, operator);
    }

    #[has_role(operator, "manager")]
    fn set_token_royalty(
        e: &Env,
        token_id: u32,
        receiver: &Address,
        basis_points: u32,
        operator: &Address,
    ) {
        Self::Impl::set_token_royalty(e, token_id, receiver, basis_points, operator);
    }

    #[has_role(operator, "manager")]
    fn remove_token_royalty(e: &Env, token_id: u32, operator: &Address) {
        Self::Impl::remove_token_royalty(e, token_id, operator);
    }

    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
        Self::Impl::royalty_info(e, token_id, sale_price)
    }
}
