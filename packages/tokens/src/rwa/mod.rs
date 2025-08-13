//! # Real World Asset (RWA) Token Contract Module
//!
//! Implements utilities for handling Real World Asset tokens in a Soroban
//! contract. This module is based on the T-REX (Token for Regulated Exchanges)
//! standard, providing comprehensive functionality for compliant security
//! tokens.
//!
//! ## Design Overview
//!
//! RWA tokens are security tokens that represent real-world assets and must
//! comply with various regulations. This module provides:
//!
//! - **Identity Management**: Integration with identity registries for KYC/AML
//! - **Compliance Framework**: Modular compliance rules and validation
//! - **Transfer Controls**: Sophisticated transfer restrictions and validations
//! - **Token Lifecycle**: Minting, burning, freezing, and recovery mechanisms
//! - **Batch Operations**: Efficient bulk operations for administrative tasks
//!
//! ## Key Features
//!
//! - **Regulatory Compliance**: Built-in support for compliance rules and
//!   identity verification
//! - **Freezing Mechanisms**: Address-level and partial token freezing
//!   capabilities
//! - **Recovery System**: Lost wallet recovery for verified investors
//! - **Pausable Operations**: Emergency pause functionality for the entire
//!   token
//! - **Agent-based Administration**: Role-based access control for
//!   administrative functions
//!
//! ## Modules
//!
//! - **Claim Topics and Issuers**: Management of trusted claim issuers and
//!   topics
//! - **Compliance**: Modular compliance rules and validation framework
//! - **Identity Claims**: Integration with identity registries for KYC/AML
//! - **Identity Verifier**: Add-on module for establishing the connection
//!   between RWA token and identity registry
//! - **Identity Storage Registry**: Registry for storing all the information
//!   necessary for identities

pub mod storage;

use soroban_sdk::{contracterror, Address, Env, Symbol};
use stellar_contract_utils::pausable::Pausable;

/// Real World Asset Token Trait
///
/// The `RWA` trait defines the core functionality for Real World Asset tokens,
/// implementing the T-REX standard for regulated securities. It provides a
/// comprehensive interface for managing compliant token transfers, identity
/// verification, compliance rules, and administrative controls.
///
/// This trait extends basic token functionality with regulatory features
/// required for security tokens, including:
/// - Identity registry integration for KYC/AML compliance
/// - Modular compliance framework for transfer validation
/// - Freezing mechanisms for regulatory enforcement
/// - Recovery system for lost wallet scenarios
/// - Administrative controls for token management
pub trait RWA: Pausable {
    // ################## CORE TOKEN FUNCTIONS ##################

    /// Returns the total amount of tokens in existence.
    fn total_supply(e: &Env) -> i128;

    /// Returns the balance of the specified address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `id` - The address to query the balance of.
    fn balance_of(e: &Env, id: Address) -> i128;

    /// Returns the amount which spender is still allowed to withdraw from
    /// owner.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address of the account owning tokens.
    /// * `spender` - The address of the account able to transfer the tokens.
    fn allowance(e: &Env, owner: Address, spender: Address) -> i128;

    /// Transfers amount tokens from the caller's account to the to account.
    /// Requires compliance validation and identity verification.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the tokens.
    /// * `amount` - The amount of tokens to transfer.
    fn transfer(e: &Env, from: Address, to: Address, amount: i128);

    /// Transfers amount tokens from the from account to the to account using
    /// the allowance mechanism. Requires compliance validation and identity
    /// verification.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address performing the transfer.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the tokens.
    /// * `amount` - The amount of tokens to transfer.
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128);

    /// Sets amount as the allowance of spender over the caller's tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address of the account owning tokens.
    /// * `spender` - The address of the account able to transfer the tokens.
    /// * `amount` - The amount of tokens to approve.
    /// * `live_until_ledger` - The ledger number until which the allowance is
    ///   valid.
    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32);

    /// Forces a transfer of tokens between two whitelisted wallets.
    /// This function can only be called by an agent of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The number of tokens to transfer.
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128);

    /// Mints tokens to a wallet. Tokens can only be minted to verified
    /// addresses. This function can only be called by an agent of the
    /// token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - Address to mint the tokens to.
    /// * `amount` - Amount of tokens to mint.
    fn mint(e: &Env, to: Address, amount: i128);

    /// Burns tokens from a wallet.
    /// This function can only be called by an agent of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - Address to burn the tokens from.
    /// * `amount` - Amount of tokens to burn.
    fn burn(e: &Env, user_address: Address, amount: i128);

    /// Recovery function used to force transfer tokens from a lost wallet
    /// to a new wallet for an investor.
    /// This function can only be called by an agent of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `lost_wallet` - The wallet that the investor lost.
    /// * `new_wallet` - The newly provided wallet for token transfer.
    /// * `investor_onchain_id` - The onchain ID of the investor asking for
    ///   recovery.
    fn recovery_address(
        e: &Env,
        lost_wallet: Address,
        new_wallet: Address,
        investor_onchain_id: Address,
    ) -> bool;

    // ################## METADATA FUNCTIONS ##################

    /// Returns the name of the token.
    fn name(e: &Env) -> Symbol;

    /// Returns the symbol of the token.
    fn symbol(e: &Env) -> Symbol;

    /// Returns the number of decimals used to get its user representation.
    fn decimals(e: &Env) -> u8;

    /// Returns the version of the token (T-REX version).
    fn version(e: &Env) -> Symbol;

    /// Returns the address of the onchain ID of the token.
    fn onchain_id(e: &Env) -> Address;

    /// Sets the token name. Only the owner can call this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `name` - The name of the token to set.
    fn set_name(e: &Env, name: Symbol);

    /// Sets the token symbol. Only the owner can call this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `symbol` - The token symbol to set.
    fn set_symbol(e: &Env, symbol: Symbol);

    /// Sets the onchain ID of the token. Only the owner can call this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `onchain_id` - The address of the onchain ID to set.
    fn set_onchain_id(e: &Env, onchain_id: Address);

    // ################## UTILITY FUNCTIONS ##################

    /// Sets the frozen status for an address.
    /// This function can only be called by an agent of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to update frozen status.
    /// * `freeze` - Frozen status of the address.
    fn set_address_frozen(e: &Env, caller: Address, user_address: Address, freeze: bool);

    /// Freezes a specified amount of tokens for a given address.
    /// This function can only be called by an agent of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to freeze tokens.
    /// * `amount` - Amount of tokens to be frozen.
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128);

    /// Unfreezes a specified amount of tokens for a given address.
    /// This function can only be called by an agent of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to unfreeze tokens.
    /// * `amount` - Amount of tokens to be unfrozen.
    fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128);

    /// Returns the freezing status of a wallet.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address of the wallet to check.
    fn is_frozen(e: &Env, user_address: Address) -> bool;

    /// Returns the amount of tokens that are partially frozen on a wallet.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address of the wallet to check.
    fn get_frozen_tokens(e: &Env, user_address: Address) -> i128;

    // ################## COMPLIANCE AND IDENTITY FUNCTIONS ##################

    /// Sets the Identity Registry for the token.
    /// Only the owner can call this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_registry` - The address of the Identity Registry to set.
    fn set_identity_registry(e: &Env, identity_registry: Address);

    /// Sets the compliance contract of the token.
    /// Only the owner can call this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `compliance` - The address of the compliance contract to set.
    fn set_compliance(e: &Env, compliance: Address);

    /// Returns the Identity Registry linked to the token.
    fn identity_registry(e: &Env) -> Address;

    /// Returns the Compliance contract linked to the token.
    fn compliance(e: &Env) -> Address;

    // ################## BATCH OPERATIONS ##################

    // TODO: what is our strategy for batch operations? Will we have a batch version
    // of each function, or will we craft a 'batcher` function? Leave it empty for
    // now
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RWAError {
    /// Indicates an error related to insufficient balance for the operation.
    InsufficientBalance = 300,
    /// Indicates a failure with the allowance mechanism.
    InsufficientAllowance = 301,
    /// Indicates an invalid value for live_until_ledger when setting allowance.
    InvalidLiveUntilLedger = 302,
    /// Indicates an error when an input must be >= 0.
    LessThanZero = 303,
    /// Indicates overflow when performing mathematical operations.
    MathOverflow = 304,
    /// Indicates access to uninitialized metadata.
    UnsetMetadata = 305,
    /// Indicates the address is frozen and cannot perform operations.
    AddressFrozen = 307,
    /// Indicates insufficient free tokens (due to partial freezing).
    InsufficientFreeTokens = 308,
    /// Indicates the identity registry is not set.
    IdentityRegistryNotSet = 309,
    /// Indicates the compliance contract is not set.
    ComplianceNotSet = 310,
    /// Indicates the address is not verified in the identity registry.
    AddressNotVerified = 311,
    /// Indicates the transfer does not comply with the compliance rules.
    TransferNotCompliant = 312,
    /// Indicates unauthorized access (not an agent or owner).
    Unauthorized = 313,
    /// Indicates the onchain ID is not set.
    OnchainIdNotSet = 314,
    /// Indicates invalid recovery parameters.
    InvalidRecoveryParams = 315,
    /// Indicates recovery failed.
    RecoveryFailed = 316,
    /// Indicates mismatched array lengths in batch operations.
    ArrayLengthMismatch = 317,
    /// Indicates empty arrays in batch operations.
    EmptyArrays = 318,
    /// Indicates the maximum batch size was exceeded.
    BatchSizeExceeded = 319,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const BALANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_TTL_THRESHOLD: u32 = BALANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const ALLOWANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const ALLOWANCE_TTL_THRESHOLD: u32 = ALLOWANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const FROZEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const FROZEN_TTL_THRESHOLD: u32 = FROZEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const INSTANCE_EXTEND_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Maximum number of addresses that can be processed in a single batch
/// operation
pub const MAX_BATCH_SIZE: u32 = 100;

// ################## EVENTS ##################

/// Emits an event indicating token information has been updated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The new name of the token.
/// * `symbol` - The new symbol of the token.
/// * `decimals` - The decimals of the token.
/// * `version` - The version of the token.
/// * `onchain_id` - The address of the onchain ID.
///
/// # Events
///
/// * topics - `["token_info_updated", name: Symbol, symbol: Symbol]`
/// * data - `[decimals: u8, version: Symbol, onchain_id: Address]`
pub fn emit_token_information_updated(
    e: &Env,
    name: &Symbol,
    symbol: &Symbol,
    decimals: u32,
    version: &Symbol,
    onchain_id: &Address,
) {
    let topics = (soroban_sdk::symbol_short!("token_upd"), name, symbol);
    e.events().publish(topics, (decimals, version, onchain_id))
}

/// Emits an event indicating the Identity Registry has been set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `identity_registry` - The address of the Identity Registry.
///
/// # Events
///
/// * topics - `["identity_registry_added", identity_registry: Address]`
/// * data - `[]`
pub fn emit_identity_registry_added(e: &Env, identity_registry: &Address) {
    let topics = (soroban_sdk::symbol_short!("idreg_add"), identity_registry);
    e.events().publish(topics, ())
}

/// Emits an event indicating the Compliance contract has been set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `compliance` - The address of the Compliance contract.
///
/// # Events
///
/// * topics - `["compliance_added", compliance: Address]`
/// * data - `[]`
pub fn emit_compliance_added(e: &Env, compliance: &Address) {
    let topics = (soroban_sdk::symbol_short!("comp_add"), compliance);
    e.events().publish(topics, ())
}

/// Emits an event indicating a successful recovery.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `lost_wallet` - The address of the lost wallet.
/// * `new_wallet` - The address of the new wallet.
/// * `investor_onchain_id` - The address of the investor's onchain ID.
///
/// # Events
///
/// * topics - `["recovery_success", lost_wallet: Address, new_wallet: Address]`
/// * data - `[investor_onchain_id: Address]`
pub fn emit_recovery_success(
    e: &Env,
    lost_wallet: &Address,
    new_wallet: &Address,
    investor_onchain_id: &Address,
) {
    let topics = (soroban_sdk::symbol_short!("recovery"), lost_wallet, new_wallet);
    e.events().publish(topics, investor_onchain_id)
}

/// Emits an event indicating an address has been frozen or unfrozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address that is affected.
/// * `is_frozen` - The freezing status of the wallet.
/// * `owner` - The address of the agent who called the function.
///
/// # Events
///
/// * topics - `["address_frozen", user_address: Address, is_frozen: bool]`
/// * data - `[owner: Address]`
pub fn emit_address_frozen(e: &Env, user_address: &Address, is_frozen: bool, owner: &Address) {
    let topics = (soroban_sdk::symbol_short!("addr_frz"), user_address, is_frozen);
    e.events().publish(topics, owner)
}

/// Emits an event indicating tokens have been frozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address where tokens are frozen.
/// * `amount` - The amount of tokens that are frozen.
///
/// # Events
///
/// * topics - `["tokens_frozen", user_address: Address]`
/// * data - `[amount: i128]`
pub fn emit_tokens_frozen(e: &Env, user_address: &Address, amount: i128) {
    let topics = (soroban_sdk::symbol_short!("tkn_frz"), user_address);
    e.events().publish(topics, amount)
}

/// Emits an event indicating tokens have been unfrozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address where tokens are unfrozen.
/// * `amount` - The amount of tokens that are unfrozen.
///
/// # Events
///
/// * topics - `["tokens_unfrozen", user_address: Address]`
/// * data - `[amount: i128]`
pub fn emit_tokens_unfrozen(e: &Env, user_address: &Address, amount: i128) {
    let topics = (soroban_sdk::symbol_short!("tkn_unfrz"), user_address);
    e.events().publish(topics, amount)
}

/// Emits an event indicating a transfer of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens transferred.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    let topics = (soroban_sdk::symbol_short!("transfer"), from, to);
    e.events().publish(topics, amount)
}

/// Emits an event indicating an allowance was set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to spender.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
///
/// # Events
///
/// * topics - `["approve", owner: Address, spender: Address]`
/// * data - `[amount: i128, live_until_ledger: u32]`
pub fn emit_approve(
    e: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
    live_until_ledger: u32,
) {
    let topics = (soroban_sdk::symbol_short!("approve"), owner, spender);
    e.events().publish(topics, (amount, live_until_ledger))
}

/// Emits an event indicating a mint of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens minted.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[amount: i128]`
pub fn emit_mint(e: &Env, to: &Address, amount: i128) {
    let topics = (soroban_sdk::symbol_short!("mint"), to);
    e.events().publish(topics, amount)
}

/// Emits an event indicating a burn of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address from which tokens were burned.
/// * `amount` - The amount of tokens burned.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[amount: i128]`
pub fn emit_burn(e: &Env, from: &Address, amount: i128) {
    let topics = (soroban_sdk::symbol_short!("burn"), from);
    e.events().publish(topics, amount)
}
