//! RWA Claim Issuer Example Contract.
//!
//! Implements the [`ClaimIssuer`] trait using Ed25519 signatures (scheme 101).
//! Authorized signing keys are managed by the contract owner via `allow_key`
//! and `remove_key`. The `is_claim_valid` function verifies that:
//!
//! 1. The scheme is Ed25519 (101).
//! 2. The signing key is authorized for the given claim topic.
//! 3. The claim has not expired (using encoded `valid_until` metadata).
//! 4. The claim has not been individually revoked.
//! 5. The Ed25519 signature over the canonical claim message is valid.

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Bytes, Env};
use stellar_access::ownable::{self, Ownable};
use stellar_macros::only_owner;
use stellar_tokens::rwa::claim_issuer::{
    allow_key, is_claim_expired, is_claim_revoked, is_key_allowed_for_registry,
    is_key_allowed_for_topic, remove_key, ClaimIssuer, ClaimIssuerError, Ed25519Verifier,
    SignatureVerifier,
};

/// Scheme identifier for Ed25519 signatures.
pub const ED25519_SCHEME: u32 = 101;

#[contract]
pub struct ClaimIssuerContract;

#[contractimpl]
impl ClaimIssuerContract {
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }

    pub fn is_key_allowed(e: &Env, public_key: Bytes, registry: Address, claim_topic: u32) -> bool {
        is_key_allowed_for_topic(e, &public_key, ED25519_SCHEME, claim_topic)
            && is_key_allowed_for_registry(e, &public_key, ED25519_SCHEME, &registry)
    }

    #[only_owner]
    pub fn allow_key(e: &Env, public_key: Bytes, registry: Address, claim_topic: u32) {
        allow_key(e, &public_key, &registry, ED25519_SCHEME, claim_topic);
    }

    #[only_owner]
    pub fn remove_key(e: &Env, public_key: Bytes, registry: Address, claim_topic: u32) {
        remove_key(e, &public_key, &registry, ED25519_SCHEME, claim_topic);
    }
}

#[contractimpl]
impl ClaimIssuer for ClaimIssuerContract {
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        scheme: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) {
        if scheme != ED25519_SCHEME {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch);
        }

        let signature_data = Ed25519Verifier::extract_signature_data(e, &sig_data);
        let public_key_bytes: Bytes = signature_data.public_key.clone().into();

        if !is_key_allowed_for_topic(e, &public_key_bytes, ED25519_SCHEME, claim_topic) {
            panic_with_error!(e, ClaimIssuerError::NotAllowed);
        }

        if is_claim_expired(e, &claim_data) {
            panic_with_error!(e, ClaimIssuerError::NotAllowed);
        }

        if is_claim_revoked(e, &identity, claim_topic, &claim_data) {
            panic_with_error!(e, ClaimIssuerError::NotAllowed);
        }

        let message = Ed25519Verifier::build_message(e, &identity, claim_topic, &claim_data);
        Ed25519Verifier::verify(e, &message, &signature_data);
    }
}

#[contractimpl(contracttrait)]
impl Ownable for ClaimIssuerContract {}
