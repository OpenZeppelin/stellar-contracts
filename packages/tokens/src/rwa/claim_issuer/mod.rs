//! # Claim Issuer Module
//!
//! This module provides functionality for validating cryptographic claims
//! about identities. The core `ClaimIssuer` trait defines a single method
//! `is_claim_valid()` that implementors must provide:
//!
//! ```rust
//! pub trait ClaimIssuer {
//!     fn is_claim_valid(
//!         e: &Env,
//!         identity: Address,
//!         claim_topic: u32,
//!         scheme: u32,
//!         sig_data: Bytes,
//!         claim_data: Bytes,
//!     ) -> bool;
//! }
//! ```
//! The trait is intentionally minimal and unopinionated, allowing maximum
//! flexibility in implementation.
//!
//! ## Verification Schemes
//!
//! A claim issuer contract can support one or multiple verification schemes,
//! identified by the `scheme` parameter. The scheme number is contract-specific
//! and has meaning only within that particular claim issuer implementation.
//!
//! Depending on the verification scheme, the `sig_data` parameter can be
//! interpreted differently. For example:
//! - Scheme 101 might expect Ed25519 signature data (64-byte signature +
//!   32-byte public key)
//! - Scheme 102 might expect Secp256k1 signature data (64-byte signature +
//!   33-byte compressed public key)
//! - Scheme 200 might use a completely custom format
//!
//! ## Optional Helper Features
//!
//! This module provides optional helper utilities that can be used in any
//! combination as needed. These are **implementation details** and not
//! requirements:
//!
//! - **Signature Verifiers**: Pre-built verifiers for Ed25519, Secp256k1, and
//!   Secp256r1 schemes with a common `SignatureVerifier` trait structure
//! - **Key Management**: Functions for topic-specific key authorization
//! - **Claim Revocation**: Revocation tracking
//!
//! Implementors are free to use alternative structures for signature
//! verification, key management, or any other aspect of claim validation.
//!
//! ## Example Usage
//!
//! ```rust
//! use soroban_sdk::{contract, contractimpl, Address, Bytes, Env};
//! use stellar_tokens::rwa::claim_issuer::{
//!     storage::{allow_key, is_claim_revoked, is_key_allowed},
//!     ClaimIssuer,
//! };
//!
//! pub const ED25519_SCHEME_NUM: u32 = 101;
//!
//! #[contract]
//! pub struct MyContract;
//!
//! #[contractimpl]
//! pub fn __constructor(e: Env, ed25519_key: Bytes) {
//!     allow_key(&e, &ed25519_key, ED25519_SCHEME_NUM, 42);
//! }
//!
//! #[contractimpl]
//! impl ClaimIssuer for MyContract {
//!     fn is_claim_valid(
//!         e: &Env,
//!         identity: Address,
//!         claim_topic: u32,
//!         scheme: u32,
//!         sig_data: Bytes,
//!         claim_data: Bytes,
//!     ) -> bool {
//!         // scheme number has a meaning only within the claim issuer
//!         if scheme == 101 {
//!             // Extract signature data
//!             let signature_data = Ed25519Verifier::extract_signature_data(e, &sig_data);
//!
//!             // Check if the public key is authorized for this topic
//!             if !is_key_allowed(e, &signature_data.public_key.to_bytes(), scheme, claim_topic) {
//!                 return false;
//!             }
//!             let message = Ed25519Verifier::build_message(&identity, claim_topic, &claim_data);
//!
//!             // Optionally check claim was not revoked.
//!             if is_claim_revoked(e, &identity, claim_topic, &claim_data) {
//!                 return false;
//!             }
//!
//!             // Verify the signature
//!             Ed25519Verifier::verify(e, &message, &signature_data)
//!         } else {
//!             // follow similar steps as for Ed25519Verifier or
//!             // panic if other schemes are not used at this claim issuer
//!         }
//!     }
//! }
//! ```

mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractclient, contracterror, contractevent, Address, Bytes, Env};
pub use storage::{
    allow_key, is_claim_revoked, is_key_allowed, remove_key, set_claim_revoked,
    ClaimIssuerStorageKey, Ed25519SignatureData, Ed25519Verifier, Secp256k1SignatureData,
    Secp256k1Verifier, Secp256r1SignatureData, Secp256r1Verifier,
};

/// Trait for validating claims issued by this identity to other identities.
#[contractclient(name = "ClaimIssuerClient")]
pub trait ClaimIssuer {
    /// Validates whether a claim is valid for a given identity.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim to validate.
    /// * `scheme` - The signature scheme used.
    /// * `sig_data` - The signature data as bytes: public key, signature and
    ///   other data required by the concrete signature scheme.
    /// * `claim_data` - The claim data to validate.
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        scheme: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) -> bool;
}

/// Trait for signature verification schemes.
///
/// Each signature scheme implements this trait to provide a consistent
/// interface for claim validation while allowing for scheme-specific
/// implementation details.
pub trait SignatureVerifier<const N: usize> {
    /// The signature data type for this signature scheme.
    type SignatureData;

    /// Extracts and returns the parsed signature data from the raw signature
    /// bytes.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `sig_data` - The signature data to parse.
    ///
    /// # Errors
    ///
    /// * [`ClaimIssuerError::SigDataMismatch`] - If signature data format is
    ///   invalid.
    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData;

    /// Builds the message to verify for claim signature validation.
    ///
    /// The message format is: claim_issuer || identity || claim_topic ||
    /// claim_data
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim.
    /// * `claim_data` - The claim data to validate.
    fn build_message(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> Bytes;

    /// Validates a claim signature using the parsed signature data and returns
    /// true if valid.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `message` - The claim message.
    /// * `signature_data` - The parsed signature data.
    fn verify(e: &Env, message: &Bytes, signature_data: &Self::SignatureData) -> bool;

    /// Returns the expected signature data length for this scheme.
    fn expected_sig_data_len() -> u32;
}

// ################## EVENTS ##################

/// Event emitted when a key is allowed for a scheme and claim topic.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyAllowed {
    #[topic]
    pub public_key: Bytes,
    pub scheme: u32,
    pub claim_topic: u32,
}

/// Event emitted when a key is removed from a scheme and claim topic.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyRemoved {
    #[topic]
    pub public_key: Bytes,
    pub scheme: u32,
    pub claim_topic: u32,
}

/// Emits an event when key is allowed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key involved in the operation.
/// * `scheme` - The signature scheme used.
/// * `claim_topic` - Optional claim topic for topic-specific operations.
pub fn emit_key_allowed(e: &Env, public_key: &Bytes, scheme: u32, claim_topic: u32) {
    KeyAllowed { public_key: public_key.clone(), scheme, claim_topic }.publish(e)
}

/// Emits an event for key management operations (allow/remove).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key involved in the operation.
/// * `scheme` - The signature scheme used.
/// * `claim_topic` - Optional claim topic for topic-specific operations.
pub fn emit_key_removed(e: &Env, public_key: &Bytes, scheme: u32, claim_topic: u32) {
    KeyRemoved { public_key: public_key.clone(), scheme, claim_topic }.publish(e)
}

/// Event emitted when a claim is revoked.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRevoked {
    #[topic]
    pub identity: Address,
    #[topic]
    pub claim_topic: u32,
    #[topic]
    pub revoked: bool,
    pub claim_data: Bytes,
}

/// Emits an event for a claim revocation operation.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim.
/// * `claim_data` - The claim data.
/// * `revoked` - Whether the claim should be marked as revoked.
pub fn emit_revocation_event(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    revoked: bool,
) {
    ClaimRevoked {
        identity: identity.clone(),
        claim_topic,
        claim_data: claim_data.clone(),
        revoked,
    }
    .publish(e);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimIssuerError {
    /// Signature data length does not match the expected scheme.
    SigDataMismatch = 350,
    /// The provided key is empty.
    KeyIsEmpty = 351,
    /// The key is already allowed for the specified topic.
    KeyAlreadyAllowed = 352,
    /// The specified key was not found in the allowed keys.
    KeyNotFound = 353,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const CLAIMS_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const CLAIMS_TTL_THRESHOLD: u32 = CLAIMS_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const KEYS_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const KEYS_TTL_THRESHOLD: u32 = KEYS_EXTEND_AMOUNT - DAY_IN_LEDGERS;
