use core::ops::RangeBounds;

use soroban_sdk::{contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env};

use crate::rwa::claim_issuer::ClaimIssuerError;

/// Storage keys for claim issuer key management.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimIssuerStorageKey {
    /// Allows signing for all topics.
    UniversalKey(Bytes),
    /// Allows signing for a specific topic.
    TopicKey(Bytes, u32),
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

/// Trait for signature verification schemes.
///
/// Each signature scheme implements this trait to provide a consistent
/// interface for claim validation while allowing for scheme-specific
/// implementation details.
pub trait SignatureVerifier {
    /// The signature data type for this signature scheme.
    type SignatureData;

    /// Extracts and returns the parsed signature data from the raw signature bytes.
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

    /// Validates a claim signature using the parsed signature data and returns true if valid.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim to validate.
    /// * `claim_data` - The claim data to validate.
    /// * `signature_data` - The parsed signature data.
    fn verify_claim_with_data(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        signature_data: &Self::SignatureData,
    ) -> bool;

    /// Returns the expected signature data length for this scheme.
    fn expected_sig_data_len() -> u32;
}

/// Ed25519 signature verifier.
///
/// Expected signature data format: public_key (32 bytes) || signature (64
/// bytes)
pub struct Ed25519Verifier;

impl SignatureVerifier for Ed25519Verifier {
    type SignatureData = Ed25519SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<32> = extract_from_bytes(e, sig_data, 0..32);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 32..96);

        Ed25519SignatureData { public_key, signature }
    }

    fn verify_claim_with_data(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        signature_data: &Self::SignatureData,
    ) -> bool {
        // Build the message to verify
        let data = build_claim_message(e, identity, claim_topic, claim_data);

        // For Ed25519, convert hash to Bytes
        let msg_slice = e.crypto().keccak256(&data).to_array();
        let msg = Bytes::from_array(e, &msg_slice);

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

impl SignatureVerifier for Secp256r1Verifier {
    type SignatureData = Secp256r1SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);

        Secp256r1SignatureData { public_key, signature }
    }

    fn verify_claim_with_data(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        signature_data: &Self::SignatureData,
    ) -> bool {
        // Build the message to verify
        let data = build_claim_message(e, identity, claim_topic, claim_data);

        // For Secp256r1, use the hash digest directly
        let msg_digest = e.crypto().keccak256(&data);

        e.crypto().secp256r1_verify(
            &signature_data.public_key,
            &msg_digest,
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

impl SignatureVerifier for Secp256k1Verifier {
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

    fn verify_claim_with_data(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        signature_data: &Self::SignatureData,
    ) -> bool {
        // Build the message to verify
        let data = build_claim_message(e, identity, claim_topic, claim_data);

        // For Secp256k1, use the hash digest directly
        let msg_digest = e.crypto().keccak256(&data);

        // Recover public key and compare
        let recovered_key = e.crypto().secp256k1_recover(
            &msg_digest,
            &signature_data.signature,
            signature_data.recovery_id,
        );
        signature_data.public_key == recovered_key
    }

    fn expected_sig_data_len() -> u32 {
        133 // 65 bytes public key + 64 bytes signature + 4 bytes recovery_id
    }
}

/// Allows a public key to sign claims universally (for all topics).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to authorize.
pub fn allow_key(e: &Env, public_key: &Bytes) {
    let key = ClaimIssuerStorageKey::UniversalKey(public_key.clone());
    e.storage().persistent().set(&key, &true);
}

/// Removes a public key from universal claim signing authorization.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to remove authorization for.
pub fn remove_key(e: &Env, public_key: &Bytes) {
    let key = ClaimIssuerStorageKey::UniversalKey(public_key.clone());
    e.storage().persistent().remove(&key);
}

/// Allows a public key to sign claims for a specific topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to authorize.
/// * `claim_topic` - The specific claim topic to authorize for.
pub fn allow_key_for_claim_topic(e: &Env, public_key: &Bytes, claim_topic: u32) {
    let key = ClaimIssuerStorageKey::TopicKey(public_key.clone(), claim_topic);
    e.storage().persistent().set(&key, &true);
}

/// Removes a public key's authorization for a specific claim topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to remove authorization for.
/// * `claim_topic` - The specific claim topic to remove authorization for.
pub fn remove_key_for_claim_topic(e: &Env, public_key: &Bytes, claim_topic: u32) {
    let key = ClaimIssuerStorageKey::TopicKey(public_key.clone(), claim_topic);
    e.storage().persistent().remove(&key);
}

/// Checks if a public key has universal authorization to sign claims for all topics and returns true if authorized.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to check.
pub fn is_key_universally_allowed(e: &Env, public_key: &Bytes) -> bool {
    let universal_key = ClaimIssuerStorageKey::UniversalKey(public_key.clone());
    e.storage().persistent().has(&universal_key)
}

/// Checks if a public key is authorized for a specific claim topic and returns true if authorized.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to check.
/// * `claim_topic` - The claim topic to check authorization for.
pub fn is_key_allowed_for_topic(e: &Env, public_key: &Bytes, claim_topic: u32) -> bool {
    let topic_key = ClaimIssuerStorageKey::TopicKey(public_key.clone(), claim_topic);
    e.storage().persistent().has(&topic_key)
}

/// Checks if a public key is allowed to sign claims for a specific topic and returns true if authorized universally or for the specific topic.
///
/// This function checks both universal authorization and topic-specific
/// authorization.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to check.
/// * `claim_topic` - The claim topic to check authorization for.
pub fn is_key_allowed(e: &Env, public_key: &Bytes, claim_topic: u32) -> bool {
    // Check universal authorization first
    if is_key_universally_allowed(e, public_key) {
        return true;
    }

    // Check topic-specific authorization
    is_key_allowed_for_topic(e, public_key, claim_topic)
}

/// Builds and returns the message to verify for claim signature validation as Bytes.
///
/// The message format is: identity || claim_topic || claim_data
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim to validate.
/// * `claim_data` - The claim data to validate.
fn build_claim_message(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> Bytes {
    let mut data = identity.to_xdr(e);
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
fn extract_from_bytes<const N: usize>(
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
