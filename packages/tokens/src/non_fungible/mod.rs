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
//! - Metadata management (`name`, `symbol`, and `token_uri`)
//!
//! The following optional extensions are available:
//!
//! - *Burnable* enables token holders to destroy their non-fungible tokens.
//! - *Enumerable* allows for enumeration of all the token IDs in the contract
//!   as well as all the token IDs owned by each account.
//! - *Consecutive* is useful for efficiently minting multiple tokens in a
//!   single transaction.
//!
//! ## Compatibility and Compliance
//!
//! The ERC-721 interface is adapted to Stellar Ecosystem, facilitating
//! cross-ecosystem familiarity and ease of use, with the following differences:
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

mod extensions;
mod overrides;
mod storage;
mod utils;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

pub use extensions::{burnable, consecutive, enumerable, royalties};
pub use overrides::{Base, ContractOverrides};
// ################## TRAIT ##################
use soroban_sdk::{contracterror, Address, Env, String};
pub use storage::{ApprovalData, NFTStorageKey};
pub use utils::sequential;

/// Vanilla NonFungible Token Trait
///
/// The `NonFungibleToken` trait defines the core functionality for non-fungible
/// tokens. It provides a standard interface for managing
/// transfers and approvals associated with non-fungible tokens.
///
/// Event for `mint` is defined, but `mint` function itself is not included
/// as a method in this trait because it is not a part of the standard,
/// the function signature may change depending on the implementation.
///
/// We do provide a function [`crate::non_fungible::Base::sequential_mint`] for
/// sequential minting, and [`crate::non_fungible::Base::mint`] for
/// non-sequential minting strategies.
///
/// # Notes
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To not confuse the developers, we did not provide
/// the default implementations here, but we are providing a macro to generate
/// the default implementations for you.
///
/// When implementing [`NonFungibleToken`] trait for your Smart Contract,
/// you can follow the below example:
///
/// ```ignore
/// #[default_impl] // **IMPORTANT**: place this above `#[contractimpl]`
/// #[contractimpl]
/// impl NonFungibleToken for MyContract {
///     ContractType = {Your Contract Type Here};
///
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     /* and the macro will generate all the missing default implementations for you */
/// }
/// ```
///
/// This trait is implemented for the following Contract Types:
/// * [`crate::non_fungible::Base`] (covering the vanilla case, and compatible
///   with [`crate::non_fungible::extensions::burnable::NonFungibleBurnable`])
///   trait
/// * [`crate::non_fungible::extensions::enumerable::Enumerable`] (enabling the
///   compatibility and overrides for
///   [`crate::non_fungible::extensions::enumerable::NonFungibleEnumerable`])
///   trait, incompatible with
///   [`crate::non_fungible::extensions::burnable::NonFungibleBurnable`]) and
///   [`crate::non_fungible::extensions::consecutive::NonFungibleConsecutive`]
///   trait.
/// * [`crate::non_fungible::extensions::consecutive::Consecutive`] (enabling
///   the compatibility and overrides for
///   [`crate::non_fungible::extensions::consecutive::NonFungibleConsecutive`])
///   trait, incompatible with
///   [`crate::non_fungible::extensions::burnable::NonFungibleBurnable`]) and
///   [`crate::non_fungible::extensions::enumerable::NonFungibleEnumerable`]
///   trait.
///
/// You can find the default implementations of this trait for `Base`,
/// `Enumerable`, and `Consecutive`, by navigating to:
/// `ContractType::{method_name}`. For example, if you want to find how
/// [`NonFungibleToken::transfer`] is implemented for the `Enumerable` contract
/// type, you can find it using
/// [`crate::non_fungible::extensions::enumerable::Enumerable::transfer`].
pub trait NonFungibleToken {
    /// Helper type that allows us to override some of the functionality of the
    /// base trait based on the extensions implemented. You should use
    /// [`crate::non_fungible::Base`] as the type if you are not using
    /// [`crate::non_fungible::extensions::enumerable::Enumerable`] or
    /// [`crate::non_fungible::extensions::consecutive::Consecutive`]
    /// extensions.
    type ContractType: ContractOverrides;

    /// Returns the number of tokens owned by `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address for which the balance is being queried.
    fn balance(e: &Env, account: Address) -> u32;

    /// Returns the owner of the token with `token_id`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token ID as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    fn owner_of(e: &Env, token_id: u32) -> Address;

    /// Transfers the token with `token_id` from `from` to `to`.
    ///
    /// WARNING: Note that the caller is responsible to confirm that the
    /// recipient is capable of receiving the `Non-Fungible` or else the NFT
    /// may be permanently lost.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - Account of the sender.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token ID as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the current owner
    ///   (before calling this function) is not `from`.
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    fn transfer(e: &Env, from: Address, to: Address, token_id: u32);

    /// Transfers the token with `token_id` from `from` to `to` by using
    /// `spender`s approval.
    ///
    /// Unlike `transfer()`, which is used when the token owner initiates the
    /// transfer, `transfer_from()` allows an approved third party
    /// (`spender`) to transfer the token on behalf of the owner. This
    /// function verifies that `spender` has the necessary approval.
    ///
    /// WARNING: Note that the caller is responsible to confirm that the
    /// recipient is capable of receiving the `Non-Fungible` or else the NFT
    /// may be permanently lost.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorizing the transfer.
    /// * `from` - Account of the sender.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token ID as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the current owner
    ///   (before calling this function) is not `from`.
    /// * [`NonFungibleTokenError::InsufficientApproval`] - If the spender does
    ///   not have a valid approval.
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32);

    /// Gives permission to `approved` to transfer the token with `token_id` to
    /// another account. The approval is cleared when the token is
    /// transferred.
    ///
    /// Only a single account can be approved at a time for a `token_id`.
    /// To remove an approval, the approver can approve their own address,
    /// effectively removing the previous approved address. Alternatively,
    /// setting the `live_until_ledger` to `0` will also revoke the approval.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `approver` - The address of the approver (should be `owner` or
    ///   `operator`).
    /// * `approved` - The address receiving the approval.
    /// * `token_id` - Token ID as a number.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires. If `live_until_ledger` is `0`, the approval is revoked.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    /// * [`NonFungibleTokenError::InvalidApprover`] - If the owner address is
    ///   not the actual owner of the token.
    /// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger
    ///   number is less than the current ledger number.
    ///
    /// # Events
    ///
    /// * topics - `["approve", from: Address, to: Address]`
    /// * data - `[token_id: u32, live_until_ledger: u32]`
    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    );

    /// Approve or remove `operator` as an operator for the owner.
    ///
    /// Operators can call `transfer_from()` for any token held by `owner`,
    /// and call `approve()` on behalf of `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `operator` - Account to add to the set of authorized operators.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires. If `live_until_ledger` is `0`, the approval is revoked.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger
    ///   number is less than the current ledger number.
    ///
    /// # Events
    ///
    /// * topics - `["approve_for_all", from: Address]`
    /// * data - `[operator: Address, live_until_ledger: u32]`
    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32);

    /// Returns the account approved for the token with `token_id`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token ID as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    fn get_approved(e: &Env, token_id: u32) -> Option<Address>;

    /// Returns whether the `operator` is allowed to manage all the assets of
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `operator` - Account to be checked.
    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool;

    /// Returns the token collection name.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn name(e: &Env) -> String;

    /// Returns the token collection symbol.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn symbol(e: &Env) -> String;

    /// Returns the Uniform Resource Identifier (URI) for the token with
    /// `token_id`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token ID as a number.
    ///
    /// # Notes
    ///
    /// If the token does not exist, this function is expected to panic.
    fn token_uri(e: &Env, token_id: u32) -> String;
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum NonFungibleTokenError {
    /// Indicates a non-existent `token_id`.
    NonExistentToken = 200,
    /// Indicates an error related to the ownership over a particular token.
    /// Used in transfers.
    IncorrectOwner = 201,
    /// Indicates a failure with the `operator`s approval. Used in transfers.
    InsufficientApproval = 202,
    /// Indicates a failure with the `approver` of a token to be approved. Used
    /// in approvals.
    InvalidApprover = 203,
    /// Indicates an invalid value for `live_until_ledger` when setting
    /// approvals.
    InvalidLiveUntilLedger = 204,
    /// Indicates overflow when adding two values
    MathOverflow = 205,
    /// Indicates all possible `token_id`s are already in use.
    TokenIDsAreDepleted = 206,
    /// Indicates an invalid amount to batch mint in `consecutive` extension.
    InvalidAmount = 207,
    /// Indicates the token does not exist in owner's list.
    TokenNotFoundInOwnerList = 208,
    /// Indicates the token does not exist in global list.
    TokenNotFoundInGlobalList = 209,
    /// Indicates access to unset metadata.
    UnsetMetadata = 210,
    /// Indicates the length of the base URI exceeds the maximum allowed.
    BaseUriMaxLenExceeded = 211,
    /// Indicates the royalty amount is higher than 10_000 (100%) basis points.
    InvalidRoyaltyAmount = 212,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const BALANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_TTL_THRESHOLD: u32 = BALANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const OWNER_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const OWNER_TTL_THRESHOLD: u32 = OWNER_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const OWNERSHIP_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const OWNERSHIP_TTL_THRESHOLD: u32 = OWNERSHIP_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const TOKEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const TOKEN_TTL_THRESHOLD: u32 = TOKEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Max. number of digits in a token ID (u32)
pub const MAX_NUM_DIGITS: usize = 10;
/// Max. allowed length for a base uri.
pub const MAX_BASE_URI_LEN: usize = 200;

// ################## EVENTS ##################

/// Event emitted when a token is transferred.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub token_id: u32,
}

/// Emits an event for a transfer of a token from `from` to `to`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `from` - The sender address.
/// * `to` - The recipient address.
/// * `token_id` - The token identifier.
#[cfg(not(feature = "certora"))]
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
    Transfer { from: from.clone(), to: to.clone(), token_id }.publish(e);
}

/// Event emitted when an approval is granted.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approve {
    #[topic]
    pub approver: Address,
    #[topic]
    pub token_id: u32,
    pub approved: Address,
    pub live_until_ledger: u32,
}

/// Emits an event when `approver` enables `approved` to manage the the token
/// with `token_id` until `live_until_ledger`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `approver` - The approver address.
/// * `approved` - The approved address.
/// * `token_id` - The token identifier.
/// * `live_until_ledger` - The ledger number until which the approval is valid.
#[cfg(not(feature = "certora"))]
pub fn emit_approve(
    e: &Env,
    approver: &Address,
    approved: &Address,
    token_id: u32,
    live_until_ledger: u32,
) {
    Approve { approver: approver.clone(), token_id, approved: approved.clone(), live_until_ledger }
        .publish(e);
}

/// Event emitted when approval for all tokens is granted.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApproveForAll {
    #[topic]
    pub owner: Address,
    pub operator: Address,
    pub live_until_ledger: u32,
}

/// Emits an event when `owner` enables `operator` to manage the the token with
/// `token_id` until `live_until_ledger`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `owner` - The owner address.
/// * `operator` - The operator address.
/// * `live_until_ledger` - The ledger number until which the approval is valid.
#[cfg(not(feature = "certora"))]
pub fn emit_approve_for_all(e: &Env, owner: &Address, operator: &Address, live_until_ledger: u32) {
    ApproveForAll { owner: owner.clone(), operator: operator.clone(), live_until_ledger }
        .publish(e);
}

/// Event emitted when a token is minted.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mint {
    #[topic]
    pub to: Address,
    pub token_id: u32,
}

/// Emits an event indicating a mint of a token.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `to` - The recipient address.
/// * `token_id` - The token identifier.
#[cfg(not(feature = "certora"))]
pub fn emit_mint(e: &Env, to: &Address, token_id: u32) {
    Mint { to: to.clone(), token_id }.publish(e);
}
