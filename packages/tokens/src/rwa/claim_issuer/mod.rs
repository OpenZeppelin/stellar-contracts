//! # Claim Issuer Module
//!
//! This module provides functionality for validating cryptographic claims
//! about identities. The core `ClaimIssuer` trait is minimal, but the module
//! offers a variety of optional features that can be integrated in any
//! combination as needed:
//!
//! - **Multiple Signature Schemes**: Ed25519, Secp256k1, Secp256r1 support
//! - **Topic-Specific Key Authorization**: Keys authorized for specific claim
//!   topics
//! - **Claim Revocation**: Digest-based revocation tracking with persistent
//!   storage
//! - **Key Management**: Add, remove and query
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
//! #[contract]
//! pub struct MyContract;
//!
//! #[contractimpl]
//! pub fn __constructor(e: Env, ed25519_key: Bytes) {
//!     allow_key(&e, &ed25519_key, 42);
//! }
//!
//! #[contractimpl]
//! impl ClaimIssuer for MyContract {
//!     fn is_claim_valid(
//!         e: &Env,
//!         identity: Address,
//!         claim_topic: u32,
//!         sig_data: Bytes,
//!         claim_data: Bytes,
//!     ) -> bool {
//!         // Extract signature data and verify against stored key
//!         let signature_data = Ed25519Verifier::extract_signature_data(e, &sig_data);
//!
//!         // Check if the public key is authorized for this topic
//!         if !is_key_allowed(e, &signature_data.public_key.to_bytes(), claim_topic) {
//!             return false;
//!         }
//!         let claim_digest =
//!             Ed25519Verifier::build_claim_digest(&identity, claim_topic, &claim_data);
//!
//!         // Optionally check claim was not revoked.
//!         if is_claim_revoked(e, &claim_digest) {
//!             return false;
//!         }
//!
//!         // Verify the signature
//!         Ed25519Verifier::verify_claim_digest(e, &claim_digest, &signature_data)
//!     }
//! }
//! ```

mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{
    contractclient, contracterror, contractevent, crypto::Hash, Address, Bytes, BytesN, Env,
};
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
    /// * `sig_data` - The signature data as bytes: public key, signature and
    ///   other data required by the concrete signature scheme.
    /// * `claim_data` - The claim data to validate.
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
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

    /// Builds the message and hashes it to verify for claim signature
    /// validation.
    ///
    /// The message format is: identity || claim_topic || claim_data
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim.
    /// * `claim_data` - The claim data to validate.
    fn build_claim_digest(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
    ) -> Hash<N>;

    /// Validates a claim signature using the parsed signature data and returns
    /// true if valid.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `claim_digest` - The hash digest of the claim message.
    /// * `signature_data` - The parsed signature data.
    fn verify_claim_digest(
        e: &Env,
        claim_digest: &Hash<N>,
        signature_data: &Self::SignatureData,
    ) -> bool;

    /// Returns the expected signature data length for this scheme.
    fn expected_sig_data_len() -> u32;
}

// ################## EVENTS ##################

/// Event types for key management operations.
pub enum KeyEvent {
    /// Key was allowed for topic-specific authorization.
    Allowed,
    /// Key was removed from topic-specific authorization.
    Removed,
}

/// Event emitted when a key is allowed for a claim topic.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyAllowed {
    #[topic]
    pub public_key: Bytes,
    pub claim_topic: u32,
}

/// Event emitted when a key is removed from a claim topic.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyRemoved {
    #[topic]
    pub public_key: Bytes,
    pub claim_topic: u32,
}

/// Emits an event for key management operations (allow/remove).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `event_type` - The type of key event.
/// * `public_key` - The public key involved in the operation.
/// * `claim_topic` - Optional claim topic for topic-specific operations.
pub fn emit_key_event(e: &Env, event_type: KeyEvent, public_key: &Bytes, claim_topic: u32) {
    match event_type {
        KeyEvent::Allowed => KeyAllowed { public_key: public_key.clone(), claim_topic }.publish(e),
        KeyEvent::Removed => KeyRemoved { public_key: public_key.clone(), claim_topic }.publish(e),
    }
}

/// Event emitted when a claim is revoked.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRevoked {
    #[topic]
    pub claim_digest: BytesN<32>,
    #[topic]
    pub revoked: bool,
}

/// Emits an event for a claim revocation operation.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_digest` - The hash digest of the claim.
/// * `revoked` - Whether the claim should be marked as revoked.
pub fn emit_revocation_event(e: &Env, claim_digest: &BytesN<32>, revoked: bool) {
    ClaimRevoked { claim_digest: claim_digest.clone(), revoked }.publish(e);
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
