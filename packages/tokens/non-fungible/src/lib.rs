//! # Non-Fungible Token Contract Module.
//!
//! Implements utilities for handling non-fungible tokens in a Soroban contract.
//!
//! This module provides essential storage functionalities required for managing
//! balances, approvals, and transfers of non-fungible tokens.
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
//! - Transfers
//! - Owner and Approval management
//! - Basic metadata management (`name`, `symbol`, and `token_uri`)
//!
//! The following optional extensions are available:
//!
//! - Metadata: Provides additional information about the token, such as name,
//!   symbol, and tokenURI.
//! - Mintable: Allows authorized entities to mint new non-fungible tokens.
//! - Burnable: Enables token holders to destroy their non-fungible tokens.
//!
//! ## Compatibility and Compliance
//!
//! The ERC-721 interface is adapted to Stellar Ecosystem,
//! facilitating cross-ecosystem familiarity and ease of use,
//! with the following differences:
//!
//! - `transfer()` function is made available due to consistency with Fungible
//!   Token interface, and also it is a simpler (thus, cheaper and faster)
//!   version of `transferFrom()`, which may become handy depending on the
//!   context.
//! - `safeTransfer` mechanism is not present in the base module, (will be
//!   provided as an extension)
//! - `name()`, `symbol()` and `token_uri()` functionalities are made available
//!   to be consistent with fungible tokens as well.
//!
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

mod extensions;
mod non_fungible;
mod storage;

pub use extensions::burnable;
pub use non_fungible::{
    emit_approval, emit_approval_for_all, emit_transfer, NonFungibleToken, NonFungibleTokenClient,
    NonFungibleTokenError,
};
pub use storage::{
    approve, balance, get_approved, is_approved_for_all, owner_of, set_approval_for_all, transfer,
    transfer_from, ApprovalData, ApprovalForAllData, StorageKey, BALANCE_EXTEND_AMOUNT,
    BALANCE_TTL_THRESHOLD, DAY_IN_LEDGERS, INSTANCE_EXTEND_AMOUNT, INSTANCE_TTL_THRESHOLD,
};

mod test;
