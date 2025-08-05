use core::ops::RangeBounds;

use soroban_sdk::{panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env};

use crate::rwa::claim_issuer::ClaimIssuerError;

/// Trait for signature verification schemes.
///
/// Each signature scheme implements this trait to provide a consistent interface
/// for claim validation while allowing for scheme-specific implementation details.
pub trait SignatureVerifier {
    /// The public key type for this signature scheme.
    type PublicKey;
    /// The signature type for this signature scheme.
    type Signature;

    /// Extracts public key and signature from the signature data.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `sig_data` - The signature data to parse.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (public_key, signature).
    ///
    /// # Errors
    ///
    /// * [`ClaimIssuerError::SigDataMismatch`] if signature data format is invalid.
    fn extract_signature_components(
        e: &Env,
        sig_data: &Bytes,
    ) -> (Self::PublicKey, Self::Signature);

    /// Validates a claim signature.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim to validate.
    /// * `claim_data` - The claim data to validate.
    /// * `sig_data` - The signature data (format depends on the scheme).
    ///
    /// # Returns
    ///
    /// Returns true if the claim signature is valid.
    ///
    /// # Errors
    ///
    /// * [`ClaimIssuerError::SigDataMismatch`] if signature data format is invalid.
    fn verify_claim(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        sig_data: &Bytes,
    ) -> bool;

    /// Returns the expected signature data length for this scheme.
    fn expected_sig_data_len() -> u32;
}

/// Ed25519 signature verifier.
///
/// Expected signature data format: public_key (32 bytes) || signature (64 bytes)
pub struct Ed25519Verifier;

impl SignatureVerifier for Ed25519Verifier {
    type PublicKey = BytesN<32>;
    type Signature = BytesN<64>;

    fn extract_signature_components(
        e: &Env,
        sig_data: &Bytes,
    ) -> (Self::PublicKey, Self::Signature) {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let pub_key: BytesN<32> = extract_from_bytes(e, sig_data, 0..32);
        let sig: BytesN<64> = extract_from_bytes(e, sig_data, 32..96);
        (pub_key, sig)
    }

    fn verify_claim(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        sig_data: &Bytes,
    ) -> bool {
        let (pub_key, sig) = Self::extract_signature_components(e, sig_data);

        let data = build_claim_message(e, identity, claim_topic, claim_data);

        // For Ed25519, convert hash to Bytes
        let msg_slice = e.crypto().keccak256(&data).to_array();
        let msg = Bytes::from_array(e, &msg_slice);

        e.crypto().ed25519_verify(&pub_key, &msg, &sig);
        true
    }

    fn expected_sig_data_len() -> u32 {
        96 // 32 bytes public key + 64 bytes signature
    }
}

/// Secp256r1 signature verifier.
///
/// Expected signature data format: public_key (65 bytes) || signature (64 bytes)
pub struct Secp256r1Verifier;

impl SignatureVerifier for Secp256r1Verifier {
    type PublicKey = BytesN<65>;
    type Signature = BytesN<64>;

    fn extract_signature_components(
        e: &Env,
        sig_data: &Bytes,
    ) -> (Self::PublicKey, Self::Signature) {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let pub_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let sig: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);
        (pub_key, sig)
    }

    fn verify_claim(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        sig_data: &Bytes,
    ) -> bool {
        let (pub_key, sig) = Self::extract_signature_components(e, sig_data);

        let data = build_claim_message(e, identity, claim_topic, claim_data);

        // For Secp256r1, use the hash digest directly
        let msg_digest = e.crypto().keccak256(&data);

        e.crypto().secp256r1_verify(&pub_key, &msg_digest, &sig);
        true
    }

    fn expected_sig_data_len() -> u32 {
        129 // 65 bytes public key + 64 bytes signature
    }
}

/// Secp256k1 signature verifier.
///
/// Expected signature data format: public_key (65 bytes) || signature (64 bytes)
pub struct Secp256k1Verifier;

impl Secp256k1Verifier {
    /// Recovery ID is hardcoded to 1 for now.
    pub fn recovery_id() -> u32 {
        1
    }
}

impl SignatureVerifier for Secp256k1Verifier {
    type PublicKey = BytesN<65>;
    type Signature = BytesN<64>;

    fn extract_signature_components(
        e: &Env,
        sig_data: &Bytes,
    ) -> (Self::PublicKey, Self::Signature) {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let pub_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let sig: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);
        (pub_key, sig)
    }

    fn verify_claim(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        sig_data: &Bytes,
    ) -> bool {
        let (pub_key, sig) = Self::extract_signature_components(e, sig_data);

        let data = build_claim_message(e, identity, claim_topic, claim_data);

        // For Secp256k1, use the hash digest directly
        let msg_digest = e.crypto().keccak256(&data);

        // Recover public key and compare
        let recovered_key = e.crypto().secp256k1_recover(&msg_digest, &sig, Self::recovery_id());
        pub_key == recovered_key
    }

    fn expected_sig_data_len() -> u32 {
        129 // 65 bytes public key + 64 bytes signature
    }
}

/// Generic claim validation function that works with any signature verifier.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `verifier` - The signature verifier to use.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim to validate.
/// * `claim_data` - The claim data to validate.
/// * `sig_data` - The signature data.
///
/// # Returns
///
/// Returns true if the claim signature is valid.
pub fn verify_claim_with_verifier<V: SignatureVerifier>(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    sig_data: &Bytes,
) -> bool {
    V::verify_claim(e, identity, claim_topic, claim_data, sig_data)
}

/// Builds the message to verify for claim signature validation.
///
/// The message format is: identity || claim_topic || claim_data
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim to validate.
/// * `claim_data` - The claim data to validate.
///
/// # Returns
///
/// Returns the constructed message as Bytes.
fn build_claim_message(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> Bytes {
    let mut data = identity.to_xdr(e);
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(claim_data);
    data
}

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
