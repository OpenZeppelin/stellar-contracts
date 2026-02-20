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

// Ensure soroban-sdk's panic handler is linked for cdylib builds.
extern crate soroban_sdk;

#[cfg(feature = "fungible")]
pub mod fungible;
#[cfg(feature = "non-fungible")]
pub mod non_fungible;
#[cfg(feature = "rwa")]
pub mod rwa;
#[cfg(feature = "vault")]
pub mod vault;
