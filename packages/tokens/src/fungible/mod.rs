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

mod extensions;
mod overrides;
mod storage;
mod utils;

#[cfg(test)]
mod test;

pub use extensions::{allowlist, blocklist, burnable, capped};
pub use overrides::{Base, ContractOverrides};
use soroban_sdk::{contracterror, Address, Env, String};
pub use storage::{AllowanceData, AllowanceKey, StorageKey};
pub use utils::{sac_admin_generic, sac_admin_wrapper};

#[cfg(feature = "certora")]
pub mod specs;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

/// Vanilla Fungible Token Trait
///
/// The `FungibleToken` trait defines the core functionality for fungible
/// tokens, adhering to SEP-41. It provides a standard interface for managing
/// balances, allowances, and metadata associated with fungible tokens.
/// Additionally, this trait includes the `total_supply()` function, which is
/// not part of SEP-41 but is commonly used in token contracts.
///
/// To fully comply with the SEP-41 specification one has to implement the
/// `FungibleBurnable` trait in addition to this one. SEP-41 mandates support
/// for token burning to be considered compliant.
///
/// Event for `mint` is defined, but `mint` function itself is not included
/// as a method in this trait because it is not a part of the SEP-41 standard,
/// the function signature may change depending on the implementation.
///
/// We do provide a function [`crate::fungible::Base::mint`] for minting to
/// cover the general use case.
///
/// # Notes
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To not confuse the developers, we did not provide
/// the default implementations here, but we are providing a macro to generate
/// them.
///
/// When implementing [`crate::non_fungible::NonFungibleToken`] trait for your
/// Smart Contract, you can follow the below example:
///
/// ```ignore
/// #[default_impl] // **IMPORTANT**: place this above `#[contractimpl]`
/// #[contractimpl]
/// impl FungibleToken for MyContract {
///     ContractType = {Your Contract Type Here};
///
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     /* and the macro will generate all the missing default implementations for you */
/// }
/// ```
///
/// This trait is implemented for the following Contract Types:
/// * [`crate::fungible::Base`] (covering the vanilla case, and compatible with
///   [`crate::fungible::burnable::FungibleBurnable`]) trait
/// * [`crate::fungible::allowlist::AllowList`] (enabling the compatibility and
///   overrides for [`crate::fungible::allowlist::FungibleAllowList`]) trait,
///   incompatible with [`crate::fungible::blocklist::BlockList`] trait and
///   [`crate::rwa::RWA`] trait.
/// * [`crate::fungible::blocklist::BlockList`] (enabling the compatibility and
///   overrides for [`crate::fungible::blocklist::FungibleBlockList`]) trait,
///   incompatible with [`crate::fungible::allowlist::AllowList`] trait and
///   [`crate::rwa::RWA`] trait.
/// * [`crate::rwa::RWA`] (enabling the compatibility and overrides for
///   [`crate::rwa::RWAToken`]) trait, incompatible with
///   [`crate::fungible::allowlist::AllowList`] trait and
///   [`crate::fungible::blocklist::BlockList`] trait.
///
/// You can find the default implementations of this trait for `Base`,
/// `Allowlist`, `Blocklist` and `RWA` by navigating to:
/// `ContractType::{method_name}`.
///
/// For example, if you want to find how
/// [`FungibleToken::transfer`] is implemented for the `Allowlist` contract
/// type, you can find it using
/// [`crate::fungible::allowlist::AllowList::transfer`].
pub trait FungibleToken {
    /// Helper type that allows us to override some of the functionality of the
    /// base trait based on the extensions implemented. You should use
    /// [`crate::fungible::Base`] as the type if you are not using
    /// [`crate::fungible::allowlist::AllowList`] or
    /// [`crate::fungible::blocklist::BlockList`] extensions.
    type ContractType: ContractOverrides;

    /// Returns the total amount of tokens in circulation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn total_supply(e: &Env) -> i128;

    /// Returns the amount of tokens held by `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address for which the balance is being queried.
    fn balance(e: &Env, account: Address) -> i128;

    /// Returns the amount of tokens a `spender` is allowed to spend on behalf
    /// of an `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    fn allowance(e: &Env, owner: Address, spender: Address) -> i128;

    /// Transfers `amount` of tokens from `from` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn transfer(e: &Env, from: Address, to: Address, amount: i128);

    /// Transfers `amount` of tokens from `from` to `to` using the
    /// allowance mechanism. `amount` is then deducted from `spender`
    /// allowance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its
    ///   allowance consumed during the transfer.
    /// * `from` - The address holding the tokens which will be transferred.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    /// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
    ///   transfer more tokens than `spender` current allowance.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128);

    /// Sets the amount of tokens a `spender` is allowed to spend on behalf of
    /// an `owner`. Overrides any existing allowance set between `spender` and
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    /// * `amount` - The amount of tokens made available to `spender`.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InvalidLiveUntilLedger`] - Occurs when
    ///   attempting to set `live_until_ledger` that is less than the current
    ///   ledger number and greater than `0`.
    /// * [`FungibleTokenError::LessThanZero`] - Occurs when `amount < 0`.
    ///
    /// # Events
    ///
    /// * topics - `["approve", from: Address, spender: Address]`
    /// * data - `[amount: i128, live_until_ledger: u32]`
    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32);

    /// Returns the number of decimals used to represent amounts of this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn decimals(e: &Env) -> u32;

    /// Returns the name for this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn name(e: &Env) -> String;

    /// Returns the symbol for this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn symbol(e: &Env) -> String;
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FungibleTokenError {
    /// Indicates an error related to the current balance of account from which
    /// tokens are expected to be transferred.
    InsufficientBalance = 100,
    /// Indicates a failure with the allowance mechanism when a given spender
    /// doesn't have enough allowance.
    InsufficientAllowance = 101,
    /// Indicates an invalid value for `live_until_ledger` when setting an
    /// allowance.
    InvalidLiveUntilLedger = 102,
    /// Indicates an error when an input that must be >= 0
    LessThanZero = 103,
    /// Indicates overflow when adding two values
    MathOverflow = 104,
    /// Indicates access to uninitialized metadata
    UnsetMetadata = 105,
    /// Indicates that the operation would have caused `total_supply` to exceed
    /// the `cap`.
    ExceededCap = 106,
    /// Indicates the supplied `cap` is not a valid cap value.
    InvalidCap = 107,
    /// Indicates the Cap was not set.
    CapNotSet = 108,
    /// Indicates the SAC address was not set.
    SACNotSet = 109,
    /// Indicates a SAC address different than expected.
    SACAddressMismatch = 110,
    /// Indicates a missing function parameter in the SAC contract context.
    SACMissingFnParam = 111,
    /// Indicates an invalid function parameter in the SAC contract context.
    SACInvalidFnParam = 112,
    /// The user is not allowed to perform this operation
    UserNotAllowed = 113,
    /// The user is blocked and cannot perform this operation
    UserBlocked = 114,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const BALANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_TTL_THRESHOLD: u32 = BALANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const ALLOW_BLOCK_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const ALLOW_BLOCK_TTL_THRESHOLD: u32 = ALLOW_BLOCK_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const INSTANCE_EXTEND_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when tokens are transferred between addresses.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

/// Emits an event indicating a transfer of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
#[cfg(not(feature = "certora"))]
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    Transfer { from: from.clone(), to: to.clone(), amount }.publish(e);
}

/// Event emitted when an allowance is approved.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approve {
    #[topic]
    pub owner: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
    pub live_until_ledger: u32,
}

/// Emits an event indicating an allowance was set.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
#[cfg(not(feature = "certora"))]
pub fn emit_approve(
    e: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
    live_until_ledger: u32,
) {
    Approve { owner: owner.clone(), spender: spender.clone(), amount, live_until_ledger }
        .publish(e);
}

/// Event emitted when tokens are minted.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mint {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

/// Emits an event indicating a mint of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
#[cfg(not(feature = "certora"))]
pub fn emit_mint(e: &Env, to: &Address, amount: i128) {
    Mint { to: to.clone(), amount }.publish(e);
}
