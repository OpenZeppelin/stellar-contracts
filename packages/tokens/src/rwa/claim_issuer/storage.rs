use core::ops::RangeBounds;

use soroban_sdk::{contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env};

use crate::rwa::claim_issuer::ClaimIssuerError;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignatureScheme {
    // public keyand signature
    Ed25519(BytesN<32>, BytesN<64>),
    // public key and signature
    Secp256r1(BytesN<65>, BytesN<64>),
    // public key, signature and recovery_id
    Secp256k1(BytesN<65>, BytesN<64>, u32),
}

/// Validates a claim signature using the specified signature scheme.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim to validate.
/// * `claim_data` - The claim data to validate.
/// * `scheme` - The signature scheme to use for verification.
///
/// # Returns
///
/// Returns true if the claim signature is valid, false otherwise.
///
/// # Panics
///
/// Panics if the signature verification fails or if the signature format is
/// invalid.
pub fn is_claim_valid(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    scheme: &SignatureScheme,
) -> bool {
    // Build the message to verify: identity || claim_topic || claim_data
    let mut data = identity.to_xdr(e);
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(claim_data);

    match scheme {
        SignatureScheme::Ed25519(pub_key, sig) => {
            // For Ed25519, convert hash to Bytes
            let msg_slice = e.crypto().keccak256(&data).to_array();
            let msg = Bytes::from_array(e, &msg_slice);
            e.crypto().ed25519_verify(pub_key, &msg, sig);

            true
        }
        SignatureScheme::Secp256r1(pub_key, sig) => {
            // For Secp256r1, use the hash digest directly
            let msg_digest = e.crypto().keccak256(&data);
            e.crypto().secp256r1_verify(pub_key, &msg_digest, sig);

            true
        }
        SignatureScheme::Secp256k1(pub_key, sig, recovery_id) => {
            // For Secp256k1, use the hash digest directly
            let msg_digest = e.crypto().keccak256(&data);

            // TODO: can we extract recovery_id from signature?
            *pub_key == e.crypto().secp256k1_recover(&msg_digest, sig, *recovery_id)
        }
    }
}

// `sig_data`: pub_key || signature
pub fn is_claim_valid_ed25519(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    sig_data: &Bytes,
) -> bool {
    // 32 + 64 bytes
    if sig_data.len() != 96 {
        panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
    }
    let pub_key = extract_from_bytes(e, sig_data, 0..32);
    let sig = extract_from_bytes(e, sig_data, 32..96);
    let scheme = SignatureScheme::Ed25519(pub_key, sig);
    is_claim_valid(e, identity, claim_topic, claim_data, &scheme)
}

// `sig_data`: pub_key || signature
pub fn is_claim_valid_secp256r1(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    sig_data: &Bytes,
) -> bool {
    // 65 + 64 bytes
    if sig_data.len() != 129 {
        panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
    }

    let pub_key = extract_from_bytes(e, sig_data, 0..65);
    let sig = extract_from_bytes(e, sig_data, 65..129);
    let scheme = SignatureScheme::Secp256r1(pub_key, sig);

    is_claim_valid(e, identity, claim_topic, claim_data, &scheme)
}

// `sig_data`: pub_key || signature
pub fn is_claim_valid_secp256k1(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    sig_data: &Bytes,
) -> bool {
    // 65 + 64 bytes
    if sig_data.len() != 129 {
        panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
    }

    let pub_key = extract_from_bytes(e, sig_data, 0..65);
    let sig = extract_from_bytes(e, sig_data, 65..129);
    let scheme = SignatureScheme::Secp256k1(pub_key, sig, 1);

    is_claim_valid(e, identity, claim_topic, claim_data, &scheme)
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
