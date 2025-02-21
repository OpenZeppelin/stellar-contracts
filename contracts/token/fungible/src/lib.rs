//! # Fungible Token Contract Module.
//!
//! Implements utilities for handling fungible tokens in a Soroban contract.
//!
//! This module provides essential storage functionalities required for managing
//! balances, allowances, and total supply of fungible tokens.
//!
//! ## Design Overview
//!
//! This module is structured to provide flexibility to developers by splitting
//! functionalities into higher-level and lower-level operations:
//!
//! - **High-Level Functions**: These include all necessary checks,
//!   verifications, authorizations, state-changing logic, and event emissions.
//!   They simplify usage by handling core logic securely. Users can directly
//!   call these functions for typical token operations without worrying about
//!   implementation details.
//!
//! - **Low-Level Functions**: These offer granular control for developers who
//!   need to compose their own workflows. Such functions expose internal
//!   mechanisms and require the caller to handle verifications and
//!   authorizations manually.
//!
//! By offering this dual-layered approach, developers can choose between
//! convenience and customization, depending on their project requirements.
//!
//! ## Structure
//!
//! The base module includes:
//!
//! - Total supply management
//! - Transfers and allowances
//!
//! The following optional extensions are available:
//!
//! - Metadata: Provides additional information about the token, such as name,
//!   symbol, and decimals.
//! - Mintable: Allows creation of new tokens that increases the total supply.
//! - Burnable: Enables token holders to destroy their tokens, reducing the
//!   total supply.
//! - Capped: Enables the contract to set a maximum limit on the total supply.
//!
//! ## Compatibility and Compliance
//!
//! The module is designed to ensure full compatibility with SEP-0041. It also
//! closely mirrors the Ethereum ERC-20 standard, facilitating cross-ecosystem
//! familiarity and ease of use.
//!
//! Developers aiming to create SEP-41-compliant tokens can leverage the
//! `soroban_sdk::token::TokenInterface` trait available in the "soroban-sdk"
//! crate. By implementing `TokenInterface` using the helper functions provided
//! in this library, they can ensure a secure and standardized implementation.
//! Alternatively, developers can combine the implementation of both the
//! [`FungibleToken`] and [`burnable::FungibleBurnable`] traits to create tokens
//! that adhere to SEP-41 while providing greater control and extensibility.
//!
//! ## Notes for Developers
//!
//! - **Security Considerations**: While high-level functions handle necessary
//!   checks, users of low-level functions must take extra care to ensure
//!   correctness and security.
//! - **Composable Design**: The modular structure encourages developers to
//!   extend functionality by combining provided primitives or creating custom
//!   extensions.
//! - **TTL management**: This library handles the TTL of only `temporary` and
//!   `persistent` storage entries declared by the library. The `instance` TTL
//!   management is left to the implementor due to flexibility. The library
//!   exposes the sane default values for extending the TTL:
//!   `INSTANCE_TTL_THRESHOLD` and `INSTANCE_EXTEND_AMOUNT`.
#![no_std]

mod extensions;
mod fungible;
mod storage;

pub use extensions::{burnable, capped, metadata, mintable};
pub use fungible::{
    emit_approve, emit_transfer, FungibleToken, FungibleTokenClient, FungibleTokenError,
};
pub use storage::{
    allowance, allowance_data, approve, balance, do_transfer, set_allowance, spend_allowance,
    total_supply, transfer, transfer_from, update, AllowanceData, AllowanceKey, StorageKey,
    BALANCE_EXTEND_AMOUNT, BALANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT, INSTANCE_TTL_THRESHOLD,
};

mod test;
