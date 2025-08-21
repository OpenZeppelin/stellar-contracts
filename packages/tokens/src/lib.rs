//! # Stellar Tokens
//!
//! This crate provides implementations for both fungible and non-fungible
//! tokens for use in Soroban smart contracts on the Stellar network.
//!
//! ## Modules
//!
//! - `fungible`: Implementation of fungible tokens (similar to ERC-20)
//! - `non_fungible`: Implementation of non-fungible tokens (similar to ERC-721)
//!
//! Each module provides its own set of traits, functions, and extensions for
//! working with the respective token type.
#![no_std]
#![allow(deprecated)]

pub mod fungible;
pub mod non_fungible;

pub use fungible::{
    allowlist::{AllowList, FungibleAllowList},
    blocklist::{BlockList, FungibleBlockList},
    burnable::FungibleBurnable,
    sac_admin_wrapper::{DefaultSacAdminWrapper, SACAdminWrapper},
    FTBase, FungibleToken, FungibleTokenError,
};
pub use non_fungible::{
    burnable::NonFungibleBurnable,
    consecutive::Consecutive,
    enumerable::{Enumerable, NonFungibleEnumerable},
    royalties::NonFungibleRoyalties,
    NFTBase, NonFungibleToken,
};

pub mod ownable {
    pub use stellar_access::Ownable;
}
