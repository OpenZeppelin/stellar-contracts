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
//! ## Base Module and Extensions
//!
//! The base module implements:
//!
//! - Total supply management
//! - Transfers and allowances
//!
//! To extend functionality, the module supports the following optional
//! features:
//!
//! - Metadata: Provides additional information about the token, such as name,
//!   symbol, and decimals.
//! - Mintable: Allows authorized entities to mint new tokens and increase the
//!   total supply.
//! - Burnable: Enables token holders to destroy their tokens, reducing the
//!   total supply.
//!
//! ## Compatibility and Compliance
//!
//! The module is designed to ensure full compatibility with SEP-0041, making it
//! easy to integrate into Soroban-based applications. It also closely mirrors
//! the Ethereum ERC-20 standard, facilitating cross-ecosystem familiarity and
//! ease of use.
//!
//! ## Notes for Developers
//!
//! - **Security Considerations**: While high-level functions handle necessary
//!   checks, users of low-level functions must take extra care to ensure
//!   correctness and security.
//! - **Composable Design**: The modular structure encourages developers to
//!   extend functionality by combining provided primitives or creating custom
//!   extensions.
#![no_std]

pub mod fungible;
pub mod storage;

mod test;
