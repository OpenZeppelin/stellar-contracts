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
#![cfg_attr(feature = "certora", allow(unused_variables, unused_imports, dead_code))]
pub mod fungible;
pub mod non_fungible;
pub mod rwa;
pub mod vault;
