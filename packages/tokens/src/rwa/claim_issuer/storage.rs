use core::ops::RangeBounds;

use soroban_sdk::{
    contracttype, crypto::Hash, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env,
};

use crate::rwa::claim_issuer::{
    emit_key_event, emit_revocation_event, ClaimIssuerError, KeyEvent, SignatureVerifier,
    CLAIMS_EXTEND_AMOUNT, CLAIMS_TTL_THRESHOLD, KEYS_EXTEND_AMOUNT, KEYS_TTL_THRESHOLD,
};

/// Storage keys for claim issuer key management.
#[contracttype]
#[derive(Clone)]
pub enum ClaimIssuerStorageKey {
    /// Allows signing for a specific topic.
    TopicKey(Bytes, u32),
    /// Tracks explicitly revoked claims by claim digest
    RevokedClaim(BytesN<32>),
}

/// Signature data for Ed25519 scheme.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ed25519SignatureData {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

/// Signature data for Secp256r1 scheme.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Secp256r1SignatureData {
    pub public_key: BytesN<65>,
    pub signature: BytesN<64>,
}

/// Signature data for Secp256k1 scheme.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Secp256k1SignatureData {
    pub public_key: BytesN<65>,
    pub signature: BytesN<64>,
    pub recovery_id: u32,
}

// ====================== SIGNATURE VERIFICATION =====================

/// Ed25519 signature verifier.
///
/// Expected signature data format: public_key (32 bytes) || signature (64
/// bytes)
pub struct Ed25519Verifier;

impl SignatureVerifier<32> for Ed25519Verifier {
    type SignatureData = Ed25519SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<32> = extract_from_bytes(e, sig_data, 0..32);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 32..96);

        Ed25519SignatureData { public_key, signature }
    }

    fn build_claim_digest(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
    ) -> Hash<32> {
        let claim_message = build_claim_message(e, identity, claim_topic, claim_data);
        e.crypto().keccak256(&claim_message)
    }

    fn verify_claim_digest(
        e: &Env,
        claim_digest: &Hash<32>,
        signature_data: &Self::SignatureData,
    ) -> bool {
        // For Ed25519, convert hash digest to Bytes
        let msg = Bytes::from_slice(e, &claim_digest.to_array());

        e.crypto().ed25519_verify(&signature_data.public_key, &msg, &signature_data.signature);
        true
    }

    fn expected_sig_data_len() -> u32 {
        96 // 32 bytes public key + 64 bytes signature
    }
}

/// Secp256r1 signature verifier.
///
/// Expected signature data format: public_key (65 bytes) || signature (64
/// bytes)
pub struct Secp256r1Verifier;

impl SignatureVerifier<32> for Secp256r1Verifier {
    type SignatureData = Secp256r1SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);

        Secp256r1SignatureData { public_key, signature }
    }

    fn build_claim_digest(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
    ) -> Hash<32> {
        let claim_message = build_claim_message(e, identity, claim_topic, claim_data);
        e.crypto().keccak256(&claim_message)
    }

    fn verify_claim_digest(
        e: &Env,
        claim_digest: &Hash<32>,
        signature_data: &Self::SignatureData,
    ) -> bool {
        // For Secp256r1, use the claim digest directly
        e.crypto().secp256r1_verify(
            &signature_data.public_key,
            claim_digest,
            &signature_data.signature,
        );
        true
    }

    fn expected_sig_data_len() -> u32 {
        129 // 65 bytes public key + 64 bytes signature
    }
}

/// Secp256k1 signature verifier.
///
/// Expected signature data format: public_key (65 bytes) || signature (64
/// bytes) || recovery_id (4 bytes)
pub struct Secp256k1Verifier;

impl SignatureVerifier<32> for Secp256k1Verifier {
    type SignatureData = Secp256k1SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);

        // Extract recovery_id from the last 4 bytes
        let recovery_id_bytes = sig_data.slice(129..133);
        let recovery_id = u32::from_be_bytes([
            recovery_id_bytes.get(0).unwrap_or(0),
            recovery_id_bytes.get(1).unwrap_or(0),
            recovery_id_bytes.get(2).unwrap_or(0),
            recovery_id_bytes.get(3).unwrap_or(0),
        ]);

        Secp256k1SignatureData { public_key, signature, recovery_id }
    }

    fn build_claim_digest(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
    ) -> Hash<32> {
        let claim_message = build_claim_message(e, identity, claim_topic, claim_data);
        e.crypto().keccak256(&claim_message)
    }

    fn verify_claim_digest(
        e: &Env,
        claim_digest: &Hash<32>,
        signature_data: &Self::SignatureData,
    ) -> bool {
        // For Secp256k1, recover public key and compare
        let recovered_key = e.crypto().secp256k1_recover(
            claim_digest,
            &signature_data.signature,
            signature_data.recovery_id,
        );
        signature_data.public_key == recovered_key
    }

    fn expected_sig_data_len() -> u32 {
        // 65 bytes public key + 64 bytes signature + 4 bytes recovery_id;
        //
        // `recovery_id` usually fits in a single byte, but the argument in
        // `secp256k1_recover` is u32, that's why expecting here 4 bytes
        133
    }
}

// ====================== KEY MANAGEMENT =====================

/// Allows a public key to sign claims for a specific topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to authorize.
/// * `claim_topic` - The specific claim topic to authorize for.
///
/// # Errors
///
/// * [`ClaimIssuerError::KeyIsEmpty`] - If attempting to allow an empty key.
/// * [`ClaimIssuerError::KeyAlreadyAllowed`] - If the key is already allowed
///   for this topic.
///
/// # Events
///
/// * topics - `["key_allowed", public_key: Bytes]`
/// * data - `[claim_topic: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn allow_key(e: &Env, public_key: &Bytes, claim_topic: u32) {
    if public_key.is_empty() {
        panic_with_error!(e, ClaimIssuerError::KeyIsEmpty)
    }

    let key = ClaimIssuerStorageKey::TopicKey(public_key.clone(), claim_topic);

    if e.storage().persistent().has(&key) {
        panic_with_error!(e, ClaimIssuerError::KeyAlreadyAllowed)
    }

    e.storage().persistent().set(&key, &true);

    emit_key_event(e, KeyEvent::Allowed, public_key, claim_topic);
}

/// Removes a public key's authorization for a specific claim topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to remove authorization for.
/// * `claim_topic` - The specific claim topic to remove authorization for.
///
/// # Errors
///
/// * [`ClaimIssuerError::KeyNotFound`] - If the key is not found for this
///   topic.
///
/// # Events
///
/// * topics - `["key_removed", public_key: Bytes]`
/// * data - `[claim_topic: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn remove_key(e: &Env, public_key: &Bytes, claim_topic: u32) {
    let key = ClaimIssuerStorageKey::TopicKey(public_key.clone(), claim_topic);

    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, ClaimIssuerError::KeyNotFound)
    }

    e.storage().persistent().remove(&key);
    emit_key_event(e, KeyEvent::Removed, public_key, claim_topic);
}

/// Checks if a public key is allowed to sign claims for a specific topic and
/// returns true if authorized for the specific topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to check.
/// * `claim_topic` - The claim topic to check authorization for.
pub fn is_key_allowed(e: &Env, public_key: &Bytes, claim_topic: u32) -> bool {
    let topic_key = ClaimIssuerStorageKey::TopicKey(public_key.clone(), claim_topic);
    if e.storage().persistent().has(&topic_key) {
        e.storage().persistent().extend_ttl(&topic_key, KEYS_TTL_THRESHOLD, KEYS_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

// ====================== CLAIM REVOCATION =====================

/// Sets the revocation status for a claim using its digest.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_digest` - The hash digest of the claim message.
/// * `revoked` - Whether the claim should be marked as revoked.
///
/// # Events
///
/// * topics - `["claim_revoked", claim_digest: Hash<32>, revoked: true]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn set_claim_revoked(e: &Env, claim_digest: &BytesN<32>, revoked: bool) {
    let key = ClaimIssuerStorageKey::RevokedClaim(claim_digest.clone());
    e.storage().persistent().set(&key, &revoked);

    emit_revocation_event(e, claim_digest, revoked);
}

/// Checks if a claim has been revoked using its digest.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_digest` - The hash digest of the claim message to check.
pub fn is_claim_revoked(e: &Env, claim_digest: &BytesN<32>) -> bool {
    let key = ClaimIssuerStorageKey::RevokedClaim(claim_digest.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(&key, CLAIMS_TTL_THRESHOLD, CLAIMS_EXTEND_AMOUNT)
        })
        .unwrap_or_default()
}

// ====================== HELPERS =====================

/// Builds and returns the message to verify for claim signature validation as
/// Bytes.
///
/// The message format is: claim issuer || identity || claim_topic || claim_data
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim to validate.
/// * `claim_data` - The claim data to validate.
pub fn build_claim_message(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
) -> Bytes {
    let mut data = e.current_contract_address().to_xdr(e);
    data.append(&identity.to_xdr(e));
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(claim_data);
    data
}

/// Extracts and returns a fixed-size array as BytesN<N> from a Bytes object.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `data` - The Bytes object to extract from.
/// * `r` - The range of bytes to extract.
pub fn extract_from_bytes<const N: usize>(
    e: &Env,
    data: &Bytes,
    r: impl RangeBounds<u32>,
) -> BytesN<N> {
    let buf = data.slice(r).to_buffer::<N>();
    let src = buf.as_slice();
    let mut items = [0u8; N];
    items.copy_from_slice(src);
    BytesN::<N>::from_array(e, &items)
}
