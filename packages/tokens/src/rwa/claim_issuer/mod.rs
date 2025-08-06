//! # Claim Issuer Module
//!
//! This module provides functionality for issuing and validating cryptographic
//! claims about identities. It supports multiple signature schemes (Ed25519,
//! Secp256k1, Secp256r1) and allows keys to be authorized either universally or
//! for specific claim topics.
//!
//! ## Example Usage
//!
//! ```rust
//! use soroban_sdk::{contract, contractimpl, Address, Bytes, Env};
//! use stellar_tokens::rwa::claim_issuer::{
//!     storage::{allow_key_for_claim_topic, build_claim_message, is_claim_revoked, is_key_allowed_for_topic},
//!     ClaimIssuer,
//! };
//!
//! #[contract]
//! pub struct MyContract;
//!
//! #[contractimpl]
//! pub fn __constructor(e: Env, ed25519_key: Bytes) {
//!     allow_key_for_claim_topic(&e, &ed25519_key, 42);
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
//!         if !is_key_allowed_for_topic(e, &signature_data.public_key.to_bytes(), claim_topic) {
//!             return false;
//!         }
//!         let claim_message = build_claim_message(&identity, claim_topic, &claim_data);
//!         let claim_digest = e.crypto().keccak256(&claim_message);
//!
//!         // Optionally check claim was not revoked.
//!         if is_claim_revoked(e, &identity, claim_topic, &claim_data) {
//!             return false;
//!         }
//!
//!         // Verify the signature
//!         Ed25519Verifier::verify_claim_digest(
//!             e,
//!             &claim_digest,
//!             &signature_data,
//!         )
//!     }
//! }
//! ```

mod storage;
mod test;

use soroban_sdk::crypto::Hash;
use soroban_sdk::{contractclient, contracterror, Address, Bytes, Env};
pub use storage::*;

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
    /// * `sig_data` - The signature data.
    /// * `claim_data` - The claim data to validate.
    ///
    /// # Returns
    ///
    /// Returns true if the claim is valid, false otherwise.
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

// ################## ERRORS ##################

// TODO: correct enumeration and move up to higher level
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimIssuerError {
    SigDataMismatch = 1,
}
