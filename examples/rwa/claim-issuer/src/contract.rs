//! RWA Claim Issuer Example Contract.
//!
//! Implements the [`ClaimIssuer`] trait using Ed25519 signatures (scheme 101).
//! Authorized signing keys are managed by the contract admin via `allow_key`
//! and `remove_key`. The `is_claim_valid` function verifies that:
//!
//! 1. The scheme is Ed25519 (101).
//! 2. The signing key is authorized for the given claim topic.
//! 3. The claim has not expired (using encoded `valid_until` metadata).
//! 4. The claim has not been individually revoked.
//! 5. The Ed25519 signature over the canonical claim message is valid.

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, Bytes, Env, Symbol, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::claim_issuer::{
    allow_key, is_claim_expired, is_claim_revoked, is_key_allowed_for_registry,
    is_key_allowed_for_topic, remove_key, ClaimIssuer, ClaimIssuerError, Ed25519Verifier,
    SignatureVerifier,
};

/// Scheme identifier for Ed25519 signatures.
pub const ED25519_SCHEME: u32 = 101;

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct ClaimIssuerContract;

#[contractimpl]
impl ClaimIssuerContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }

    pub fn is_key_allowed(e: &Env, public_key: Bytes, registry: Address, claim_topic: u32) -> bool {
        is_key_allowed_for_topic(e, &public_key, ED25519_SCHEME, claim_topic)
            && is_key_allowed_for_registry(e, &public_key, ED25519_SCHEME, &registry)
    }

    /// Authorizes an Ed25519 public key to sign claims for a specific topic
    /// at a given registry (claim topics and issuers contract).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `public_key` - The 32-byte Ed25519 public key to authorize.
    /// * `registry` - The claim topics and issuers contract that lists this
    ///   issuer as trusted.
    /// * `claim_topic` - The topic the key is authorized to sign claims for.
    /// * `operator` - The address of the operator.
    #[only_role(operator, "manager")]
    pub fn allow_key(
        e: &Env,
        public_key: Bytes,
        registry: Address,
        claim_topic: u32,
        operator: Address,
    ) {
        allow_key(e, &public_key, &registry, ED25519_SCHEME, claim_topic);
    }

    /// Revokes authorization for an Ed25519 public key for a specific topic
    /// at a given registry.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `public_key` - The 32-byte Ed25519 public key to revoke.
    /// * `registry` - The claim topics and issuers contract.
    /// * `claim_topic` - The topic to revoke authorization for.
    /// * `operator` - The address of the operator.
    #[only_role(operator, "manager")]
    pub fn remove_key(
        e: &Env,
        public_key: Bytes,
        registry: Address,
        claim_topic: u32,
        operator: Address,
    ) {
        remove_key(e, &public_key, &registry, ED25519_SCHEME, claim_topic);
    }
}

#[contractimpl]
impl ClaimIssuer for ClaimIssuerContract {
    /// Validates a claim. Panics if the claim is invalid for any reason.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity` - The onchain identity address the claim is about.
    /// * `claim_topic` - The topic of the claim.
    /// * `scheme` - Must be [`ED25519_SCHEME`] (101).
    /// * `sig_data` - 96 bytes: 32-byte Ed25519 public key || 64-byte
    ///   signature.
    /// * `claim_data` - The claim data, which must encode expiration metadata
    ///   (see [`stellar_tokens::rwa::claim_issuer::encode_claim_data_expiration`]).
    ///
    /// # Errors
    ///
    /// * [`ClaimIssuerError::SigDataMismatch`] - When `scheme` is not 101 or
    ///   `sig_data` is not 96 bytes.
    /// * [`ClaimIssuerError::NotAllowed`] - When the signing key is not
    ///   authorized for the given topic.
    /// * panics with a contract error when the claim has expired, is revoked,
    ///   or the signature is invalid.
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
impl AccessControl for ClaimIssuerContract {}
