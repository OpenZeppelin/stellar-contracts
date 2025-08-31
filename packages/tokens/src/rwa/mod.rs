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
//! - **RBAC**: Role-based access control allows to define and set custom
//!   privileges for the administrative functions based on the needs of the
//!   token/project.
//!
//! ## Modules
//!
//! - **Claim Topics and Issuers**: Management of trusted claim issuers and
//!   topics
//! - **Compliance**: Modular compliance rules and validation framework
//! - **Identity Claims**: Integration with identity registries for KYC/AML
//! - **Identity Verifier**: Trait for establishing the connection between RWA
//!   token and identity registry
//! - **Identity Storage Registry**: Registry for storing all the information
//!   necessary for identities

pub mod claim_issuer;
pub mod claim_topics_and_issuers;
pub mod compliance;
pub mod identity_claims;
pub mod identity_registry_storage;
pub mod storage;
pub mod test;
pub mod utils;

use soroban_sdk::{contracterror, symbol_short, Address, Env, Symbol};
use stellar_contract_utils::pausable::Pausable;
pub use storage::RWA;

use crate::fungible::FungibleToken;

/// Real World Asset Token Trait
///
/// The `RWAToken` trait defines the core functionality for Real World Asset
/// tokens, implementing the T-REX standard for regulated securities. It
/// provides a comprehensive interface for managing compliant token transfers,
/// identity verification, compliance rules, and administrative controls.
///
/// This trait extends basic fungible token functionality with regulatory
/// features required for security tokens, including:
/// - Identity registry integration for KYC/AML compliance
/// - Modular compliance framework for transfer validation
/// - Freezing mechanisms for regulatory enforcement
/// - Recovery system for lost wallet scenarios
/// - Administrative controls for token management
pub trait RWAToken: Pausable + FungibleToken<ContractType = RWA> {
    // ################## CORE TOKEN FUNCTIONS ##################

    /// Forces a transfer of tokens between two whitelisted wallets.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The number of tokens to transfer.
    ///
    /// # Errors
    ///
    /// * [`RWAError::InsufficientBalance`] - When the sender has insufficient
    ///   balance.
    /// * [`RWAError::LessThanZero`] - When the amount is negative.
    fn forced_transfer(e: &Env, operator: Address, from: Address, to: Address, amount: i128);

    /// Mints tokens to a wallet. Tokens can only be minted to verified
    /// addresses. This function can only be called by the operator with
    /// necessary privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `to` - Address to mint the tokens to.
    /// * `amount` - Amount of tokens to mint.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is
    ///   not configured.
    /// * [`RWAError::AddressNotVerified`] - When the recipient address is not
    ///   verified.
    /// * [`RWAError::AddressFrozen`] - When the recipient address is frozen.
    /// * [`PausableError::EnforcedPause`] - When the contract is paused.
    fn mint(e: &Env, operator: Address, to: Address, amount: i128);

    /// Burns tokens from a wallet.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - Address to burn the tokens from.
    /// * `amount` - Amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * [`RWAError::AddressFrozen`] - When the address is frozen.
    fn burn(e: &Env, user_address: Address, amount: i128);

    /// Recovery function used to force transfer tokens from a lost wallet
    /// to a new wallet for an investor.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `lost_wallet` - The wallet that the investor lost.
    /// * `new_wallet` - The newly provided wallet for token transfer.
    /// * `investor_onchain_id` - The onchain ID of the investor asking for
    ///   recovery.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is
    ///   not configured.
    /// * [`RWAError::AddressNotVerified`] - When the new wallet is not
    ///   verified.
    /// * [`RWAError::RecoveryFailed`] - When recovery parameters are invalid.
    fn recovery_address(
        e: &Env,
        operator: Address,
        lost_wallet: Address,
        new_wallet: Address,
        investor_onchain_id: Address,
    ) -> bool;

    /// Sets the frozen status for an address. Frozen addresses cannot send or
    /// receive tokens. This function can only be called by the operator
    /// with necessary privileges. RBAC checks are expected to be enforced
    /// on the `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `user_address` - The address for which to update frozen status.
    /// * `freeze` - Frozen status of the address.
    fn set_address_frozen(e: &Env, operator: Address, user_address: Address, freeze: bool);

    /// Freezes a specified amount of tokens for a given address.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `user_address` - The address for which to freeze tokens.
    /// * `amount` - Amount of tokens to be frozen.
    ///
    /// # Errors
    ///
    /// * [`RWAError::LessThanZero`] - When the amount is negative.
    /// * [`RWAError::InsufficientBalance`] - When the address has insufficient
    ///   balance.
    fn freeze_partial_tokens(e: &Env, operator: Address, user_address: Address, amount: i128);

    /// Unfreezes a specified amount of tokens for a given address.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `user_address` - The address for which to unfreeze tokens.
    /// * `amount` - Amount of tokens to be unfrozen.
    ///
    /// # Errors
    ///
    /// * [`RWAError::LessThanZero`] - When the amount is negative.
    /// * [`RWAError::InsufficientFreeTokens`] - When there are insufficient
    ///   frozen tokens to unfreeze.
    fn unfreeze_partial_tokens(e: &Env, operator: Address, user_address: Address, amount: i128);

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

    // ################## METADATA FUNCTIONS ##################

    /// Returns the version of the token (T-REX version).
    ///
    /// # Errors
    ///
    /// * [`RWAError::UnsetMetadata`] - When the token metadata is not
    ///   initialized.
    fn version(e: &Env) -> Symbol;

    /// Returns the address of the onchain ID of the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::OnchainIdNotSet`] - When the onchain ID is not set.
    fn onchain_id(e: &Env) -> Address;

    // ################## COMPLIANCE AND IDENTITY FUNCTIONS ##################

    /// Sets the Identity Verifier for the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `identity_verifier` - The address of the Identity Verifier to set.
    fn set_identity_verifier(e: &Env, operator: Address, identity_verifier: Address);

    /// Sets the compliance contract of the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address of the operator.
    /// * `compliance` - The address of the compliance contract to set.
    fn set_compliance(e: &Env, operator: Address, compliance: Address);

    /// Returns the Identity Verifier linked to the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is
    ///   not set.
    fn identity_verifier(e: &Env) -> Address;

    /// Returns the Compliance contract linked to the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::ComplianceNotSet`] - When the compliance contract is not
    ///   set.
    fn compliance(e: &Env) -> Address;

    // ################## BATCH OPERATIONS ##################

    // TODO: will be determined in the future
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RWAError {
    /// Indicates an error related to insufficient balance for the operation.
    InsufficientBalance = 300,
    /// Indicates an error when an input must be >= 0.
    LessThanZero = 301,
    /// Indicates the address is frozen and cannot perform operations.
    AddressFrozen = 302,
    /// Indicates insufficient free tokens (due to partial freezing).
    InsufficientFreeTokens = 303,
    /// Indicates the identity verifier is not set.
    IdentityVerifierNotSet = 304,
    /// Indicates the compliance contract is not set.
    ComplianceNotSet = 305,
    /// Indicates the address is not verified in the identity registry.
    AddressNotVerified = 306,
    /// Indicates the transfer does not comply with the compliance rules.
    TransferNotCompliant = 307,
    /// Indicates the onchain ID is not set.
    OnchainIdNotSet = 308,
    /// Indicates recovery failed.
    RecoveryFailed = 309,
    /// Indicates the version is not set.
    VersionNotSet = 310,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const FROZEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const FROZEN_TTL_THRESHOLD: u32 = FROZEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;

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
/// * data - `[decimals: u8, version: &str, onchain_id: Address]`
pub fn emit_token_information_updated(
    e: &Env,
    name: Option<&Symbol>,
    symbol: Option<&Symbol>,
    decimals: Option<u32>,
    version: Option<&str>,
    onchain_id: Option<&Address>,
) {
    let topics = (Symbol::new(e, "token_updated"), name, symbol);
    e.events().publish(topics, (decimals, version, onchain_id))
}

/// Emits an event indicating the Identity Verifier has been set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `identity_verifier` - The address of the Identity Verifier.
///
/// # Events
///
/// * topics - `["identity_verifier_added", identity_verifier: Address]`
/// * data - `[]`
pub fn emit_identity_verifier_added(e: &Env, identity_verifier: &Address) {
    let topics = (Symbol::new(e, "identity_verifier_added"), identity_verifier);
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
    let topics = (Symbol::new(e, "compliance_added"), compliance);
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
    let topics = (symbol_short!("recovered"), lost_wallet, new_wallet);
    e.events().publish(topics, investor_onchain_id)
}

/// Emits an event indicating an address has been frozen or unfrozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address that is affected.
/// * `is_frozen` - The freezing status of the wallet.
/// * `owner` - The address of the who called the function.
///
/// # Events
///
/// * topics - `["address_frozen", user_address: Address, is_frozen: bool]`
/// * data - `[owner: Address]`
pub fn emit_address_frozen(e: &Env, user_address: &Address, is_frozen: bool, owner: &Address) {
    let topics = (Symbol::new(e, "address_frozen"), user_address, is_frozen);
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
    let topics = (Symbol::new(e, "tokens_frozen"), user_address);
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
    let topics = (Symbol::new(e, "tokens_unfrozen"), user_address);
    e.events().publish(topics, amount)
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
    let topics = (symbol_short!("minted"), to);
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
    let topics = (symbol_short!("burned"), from);
    e.events().publish(topics, amount)
}
