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
//!
//! ## Key Features
//!
//! - **Regulatory Compliance**: Built-in support for compliance rules and
//!   identity verification
//! - **Freezing Mechanisms**: Address-level and partial token freezing
//!   capabilities
//! - **Recovery System**: Lost/old account recovery for verified investors
//! - **Pausable Operations**: Emergency pause functionality for the entire
//!   token
//! - **RBAC**: Role-based access control allows to define and set custom
//!   privileges for the administrative functions based on the needs of the
//!   token/project.
//!
//! ## Flexibility
//!
//! RWA Token contract is designed to be flexible and extensible, allowing
//! for easy integration with variety of identity registry and compliance
//! frameworks (i.e. Merkle-Tree or Zero-Knowledge based identity
//! registries, etc.). This flexibility is achieved by the architecture
//! and loose-coupling between the modules and the token contract.
//!
//! ## Architecture
//!
//! The main RWA contract denoted by [`RWAToken`] interface, is expecting two
//! modules to be available from external contracts:
//!
//! ### Compliance Module:
//! - `fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token:
//!   Address) -> bool;`
//! - `fn can_create(e: &Env, to: Address, amount: i128, token: Address) ->
//!   bool;`
//! - `fn created(e: &Env, to: Address, amount: i128, token: Address);`
//! - `fn destroyed(e: &Env, from: Address, amount: i128, token: Address);`
//! - `fn transferred(e: &Env, from: Address, to: Address, amount: i128, token:
//!   Address);`
//!
//! ### Identity Verifier Module:
//! - `fn verify_identity(e: &Env, account: &Address);`
//!
//! Hence, the [`RWAToken`] interface also exposes the following functions to
//! set and get the necessary contracts:
//!
//! Compliance:
//! - `set_compliance(e: &Env, compliance: Address, operator: Address)`
//! - `compliance(e: &Env) -> Address`
//!
//! Identity Verifier:
//! - `set_identity_verifier(e: &Env, identity_verifier: Address, operator:
//!   Address)`
//! - `identity_verifier(e: &Env) -> Address`
//!
//! As long as these two modules and the mentioned functions are available to
//! the RWA Token contract, the [`RWAToken`] interface can be used to create a
//! compliant RWA token.
//!
//! The above mentioned functions are deliberately marked as implementation
//! details, to provide flexibility and allow for alternate approaches for
//! different business needs.
//!
//! The concrete implementation for these functions can be found in the modules
//! below. These modules act as the default implementations for the most common
//! needs.
//!
//! ## Modules
//!
//! - **Claim Topics and Issuers**: Management of trusted claim issuers and
//!   topics
//! - **Compliance**: Modular compliance rules and validation framework
//! - **Identity Claims**: Integration with identity registries for KYC/AML
//! - **Identity Registry Storage**: Registry for storing all the information
//!   necessary for identities
//! - **Identity Verifier**: Interface for the identity verification process,
//!   connects the RWA token to the identity stack
//! - **Extensions**: Optional extensions providing additional functionality
//!   like document management

pub mod claim_issuer;
pub mod claim_topics_and_issuers;
pub mod compliance;
pub mod extensions;
pub mod identity_claims;
pub mod identity_registry_storage;
pub mod identity_verifier;
pub mod storage;
pub mod utils;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{contracterror, Address, Env, String};
use stellar_contract_utils::pausable::Pausable;
pub use storage::{RWAStorageKey, RWA};

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
/// - Recovery system for lost/old account scenarios
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
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The number of tokens to transfer.
    /// * `operator` - The address of the operator.
    ///
    /// # Errors
    ///
    /// * [`RWAError::InsufficientBalance`] - When the sender has insufficient
    ///   balance.
    /// * [`RWAError::LessThanZero`] - When the amount is negative.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address);

    /// Mints tokens to a wallet. Tokens can only be minted to verified
    /// addresses. This function can only be called by the operator with
    /// necessary privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - Address to mint the tokens to.
    /// * `amount` - Amount of tokens to mint.
    /// * `operator` - The address of the operator.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerificationFailed`] - When the identity of the
    ///   recipient address cannot be verified.
    /// * [`RWAError::AddressFrozen`] - When the recipient address is frozen.
    /// * [`stellar_contract_utils::pausable::PausableError::EnforcedPause`] -
    ///   When the contract is paused.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[amount: i128]`
    fn mint(e: &Env, to: Address, amount: i128, operator: Address);

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
    /// * `operator` - The address of the operator.
    ///
    /// # Errors
    ///
    /// * [`RWAError::AddressFrozen`] - When the address is frozen.
    ///
    /// # Events
    ///
    /// * topics - `["burn", user_address: Address]`
    /// * data - `[amount: i128]`
    fn burn(e: &Env, user_address: Address, amount: i128, operator: Address);

    /// Recovery function used to force transfer tokens from a old account
    /// to a new account for an investor.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `old_account` - The wallet that the investor lost.
    /// * `new_account` - The newly provided wallet for token transfer.
    /// * `operator` - The address of the operator.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerificationFailed`] - When the identity of the
    ///   new account cannot be verified.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", old_account: Address, new_account: Address]`
    /// * data - `[amount: i128]`
    /// * topics - `["recovery", old_account: Address, new_account: Address]`
    /// * data - `[]`
    fn recover_balance(
        e: &Env,
        old_account: Address,
        new_account: Address,
        operator: Address,
    ) -> bool;

    /// Sets the frozen status for an address. Frozen addresses cannot send or
    /// receive tokens. This function can only be called by the operator
    /// with necessary privileges. RBAC checks are expected to be enforced
    /// on the `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to update frozen status.
    /// * `freeze` - Frozen status of the address.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["address_frozen", user_address: Address, is_frozen: bool,
    ///   operator: Address]`
    /// * data - `[]`
    fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address);

    /// Freezes a specified amount of tokens for a given address.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to freeze tokens.
    /// * `amount` - Amount of tokens to be frozen.
    /// * `operator` - The address of the operator.
    ///
    /// # Errors
    ///
    /// * [`RWAError::LessThanZero`] - When the amount is negative.
    /// * [`RWAError::InsufficientBalance`] - When the address has insufficient
    ///   balance.
    ///
    /// # Events
    ///
    /// * topics - `["tokens_frozen", user_address: Address]`
    /// * data - `[amount: i128]`
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address);

    /// Unfreezes a specified amount of tokens for a given address.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to unfreeze tokens.
    /// * `amount` - Amount of tokens to be unfrozen.
    /// * `operator` - The address of the operator.
    ///
    /// # Errors
    ///
    /// * [`RWAError::LessThanZero`] - When the amount is negative.
    /// * [`RWAError::InsufficientFreeTokens`] - When there are insufficient
    ///   frozen tokens to unfreeze.
    ///
    /// # Events
    ///
    /// * topics - `["tokens_unfrozen", user_address: Address]`
    /// * data - `[amount: i128]`
    fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address);

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
    /// * [`RWAError::VersionNotSet`] - When the token version is not set.
    fn version(e: &Env) -> String;

    /// Returns the address of the onchain ID of the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::OnchainIdNotSet`] - When the onchain ID is not set.
    fn onchain_id(e: &Env) -> Address;

    // ################## COMPLIANCE AND IDENTITY FUNCTIONS ##################

    /// Sets the compliance contract of the token.
    ///
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `compliance` - The address of the compliance contract to set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["compliance_set", compliance: Address]`
    /// * data - `[]`
    fn set_compliance(e: &Env, compliance: Address, operator: Address);

    /// Returns the Compliance contract linked to the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::ComplianceNotSet`] - When the compliance contract is not
    ///   set.
    fn compliance(e: &Env) -> Address;

    /// Sets the identity verifier contract of the token.
    ///
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_verifier` - The address of the identity verifier contract to
    ///   set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - ["identity_verifier_set", identity_verifier: Address]
    /// * data - `[]`
    fn set_identity_verifier(e: &Env, identity_verifier: Address, operator: Address);

    /// Returns the Identity Verifier contract linked to the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier
    ///   contract is not set.
    fn identity_verifier(e: &Env) -> Address;
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
    /// Indicates an identity cannot be verified.
    IdentityVerificationFailed = 304,
    /// Indicates the transfer does not comply with the compliance rules.
    TransferNotCompliant = 305,
    /// Indicates the mint operation does not comply with the compliance rules.
    MintNotCompliant = 306,
    /// Indicates the compliance contract is not set.
    ComplianceNotSet = 307,
    /// Indicates the onchain ID is not set.
    OnchainIdNotSet = 308,
    /// Indicates the version is not set.
    VersionNotSet = 309,
    /// Indicates the claim topics and issuers contract is not set.
    ClaimTopicsAndIssuersNotSet = 310,
    /// Indicates the identity registry storage contract is not set.
    IdentityRegistryStorageNotSet = 311,
    /// Indicates the identity verifier contract is not set.
    IdentityVerifierNotSet = 312,
    /// Indicates the old account and new account have different identities.
    IdentityMismatch = 313,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const FROZEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const FROZEN_TTL_THRESHOLD: u32 = FROZEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when token onchain ID is updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenOnchainIdUpdated {
    #[topic]
    pub onchain_id: Address,
}

/// Emits an event indicating token onchain ID is updated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `onchain_id` - The address of the onchain ID.
#[cfg(not(feature = "certora"))]
pub fn emit_token_onchain_id_updated(e: &Env, onchain_id: &Address) {
    TokenOnchainIdUpdated { onchain_id: onchain_id.clone() }.publish(e);
}

/// Event emitted when a recovery is successful.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySuccess {
    #[topic]
    pub old_account: Address,
    #[topic]
    pub new_account: Address,
}

/// Emits an event indicating a successful recovery.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_account` - The address of the old account.
/// * `new_account` - The address of the new account.
#[cfg(not(feature = "certora"))]
pub fn emit_recovery_success(e: &Env, old_account: &Address, new_account: &Address) {
    RecoverySuccess { old_account: old_account.clone(), new_account: new_account.clone() }
        .publish(e);
}

/// Event emitted when an address is frozen or unfrozen.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressFrozen {
    #[topic]
    pub user_address: Address,
    #[topic]
    pub is_frozen: bool,
}

/// Emits an event indicating an address has been frozen or unfrozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address that is affected.
/// * `is_frozen` - The freezing status of the wallet.
#[cfg(not(feature = "certora"))]
pub fn emit_address_frozen(e: &Env, user_address: &Address, is_frozen: bool) {
    AddressFrozen { user_address: user_address.clone(), is_frozen }.publish(e);
}

/// Event emitted when tokens are frozen.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokensFrozen {
    #[topic]
    pub user_address: Address,
    pub amount: i128,
}

/// Emits an event indicating tokens have been frozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address where tokens are frozen.
/// * `amount` - The amount of tokens that are frozen.
#[cfg(not(feature = "certora"))]
pub fn emit_tokens_frozen(e: &Env, user_address: &Address, amount: i128) {
    TokensFrozen { user_address: user_address.clone(), amount }.publish(e);
}

/// Event emitted when tokens are unfrozen.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokensUnfrozen {
    #[topic]
    pub user_address: Address,
    pub amount: i128,
}

/// Emits an event indicating tokens have been unfrozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The wallet address where tokens are unfrozen.
/// * `amount` - The amount of tokens that are unfrozen.
#[cfg(not(feature = "certora"))]
pub fn emit_tokens_unfrozen(e: &Env, user_address: &Address, amount: i128) {
    TokensUnfrozen { user_address: user_address.clone(), amount }.publish(e);
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
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens minted.
#[cfg(not(feature = "certora"))]
pub fn emit_mint(e: &Env, to: &Address, amount: i128) {
    Mint { to: to.clone(), amount }.publish(e);
}

/// Event emitted when tokens are burned.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Burn {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

/// Emits an event indicating a burn of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address from which tokens were burned.
/// * `amount` - The amount of tokens burned.
#[cfg(not(feature = "certora"))]
pub fn emit_burn(e: &Env, from: &Address, amount: i128) {
    Burn { from: from.clone(), amount }.publish(e);
}

/// Event emitted when compliance contract is set.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceSet {
    #[topic]
    pub compliance: Address,
}

/// Emits an event indicating the Compliance contract has been set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `compliance` - The address of the Compliance contract.
#[cfg(not(feature = "certora"))]
pub fn emit_compliance_set(e: &Env, compliance: &Address) {
    ComplianceSet { compliance: compliance.clone() }.publish(e);
}

/// Event emitted when claim topics and issuers contract is set.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTopicsAndIssuersSet {
    #[topic]
    pub claim_topics_and_issuers: Address,
}

/// Emits an event indicating the Claim Topics and Issuers contract has been
/// set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `claim_topics_and_issuers` - The address of the Claim Topics and Issuers
///   contract.
#[cfg(not(feature = "certora"))]
pub fn emit_claim_topics_and_issuers_set(e: &Env, claim_topics_and_issuers: &Address) {
    ClaimTopicsAndIssuersSet { claim_topics_and_issuers: claim_topics_and_issuers.clone() }
        .publish(e);
}

/// Event emitted when identity verifier contract is set.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityVerifierSet {
    #[topic]
    pub identity_verifier: Address,
}

/// Emits an event indicating the Identity Verifier contract has been set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `identity_verifier` - The address of the Identity Verifier contract.
#[cfg(not(feature = "certora"))]
pub fn emit_identity_verifier_set(e: &Env, identity_verifier: &Address) {
    IdentityVerifierSet { identity_verifier: identity_verifier.clone() }.publish(e);
}
