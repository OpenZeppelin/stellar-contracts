//! # Claim Issuer Contract
//!
//! Validates cryptographic claims about identities using Ed25519 signatures.
//! This contract can authorize keys for specific claim topics and track
//! claim revocations.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Bytes, Env, String};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::rwa::claim_issuer::{
    allow_key, is_claim_revoked, is_key_allowed, remove_key, set_claim_revoked, ClaimIssuer,
    Ed25519Verifier, SignatureVerifier,
};

/// Role for key administrators who can add/remove signing keys
pub const KEY_ADMIN_ROLE: soroban_sdk::Symbol = symbol_short!("KEY_ADM");
/// Role for claim administrators who can revoke claims
pub const CLAIM_ADMIN_ROLE: soroban_sdk::Symbol = symbol_short!("CLM_ADM");

#[contract]
pub struct ClaimIssuerContract;

#[contractimpl]
impl ClaimIssuer for ClaimIssuerContract {
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) -> bool {
        // Extract signature data and verify against stored key
        let signature_data = Ed25519Verifier::extract_signature_data(e, &sig_data);

        // Check if the public key is authorized for this topic
        if !is_key_allowed(e, &signature_data.public_key, claim_topic) {
            return false;
        }

        let claim_digest =
            Ed25519Verifier::build_claim_digest(e, &identity, claim_topic, &claim_data);

        // Check if claim was revoked
        if is_claim_revoked(e, &claim_digest) {
            return false;
        }

        // Verify the signature
        Ed25519Verifier::verify_claim_digest(e, &claim_digest, &signature_data)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ClaimIssuerContract {}

#[contractimpl]
impl ClaimIssuerContract {
    /// Initializes the claim issuer contract
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &admin, &KEY_ADMIN_ROLE);
        access_control::grant_role_no_auth(e, &admin, &admin, &CLAIM_ADMIN_ROLE);
    }

    /// Authorizes a key for a specific claim topic
    #[only_role(admin, "KEY_ADM")]
    pub fn allow_key(e: &Env, key: Bytes, claim_topic: u32, admin: Address) {
        allow_key(e, &key, claim_topic);
    }

    /// Removes authorization for a key and claim topic
    #[only_role(admin, "KEY_ADM")]
    pub fn remove_key(e: &Env, key: Bytes, claim_topic: u32, admin: Address) {
        remove_key(e, &key, claim_topic);
    }

    /// Checks if a key is allowed for a claim topic
    pub fn is_key_allowed(e: &Env, key: Bytes, claim_topic: u32) -> bool {
        is_key_allowed(e, &key, claim_topic)
    }

    /// Revokes a claim by its digest
    #[only_role(admin, "CLM_ADM")]
    pub fn revoke_claim(e: &Env, claim_digest: Bytes, admin: Address) {
        set_claim_revoked(e, &claim_digest, true);
    }

    /// Checks if a claim is revoked
    pub fn is_claim_revoked(e: &Env, claim_digest: Bytes) -> bool {
        is_claim_revoked(e, &claim_digest)
    }

    /// Returns the name of this claim issuer
    pub fn name(e: &Env) -> String {
        String::from_str(e, "Example Claim Issuer")
    }
}
