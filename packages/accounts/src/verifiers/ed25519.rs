/// Contract for verifying Ed25519 digital signatures.
///
/// This module provides Ed25519 signature verification functionality for
/// Stellar smart contracts.
use soroban_sdk::{Bytes, BytesN, Env};

/// Verifies an Ed25519 digital signature.
///
/// This function performs Ed25519 signature verification using the Soroban
/// cryptographic primitives. It extracts the public key from the key data,
/// parses the signature from XDR format, and verifies the signature against
/// the provided payload.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signature_payload` - The data that was signed.
/// * `public_key` - The public key (32 bytes).
/// * `signature` - The signature data (64 bytes).
///
/// # Returns
///
/// Returns `true` if the signature is valid for the given payload and public
/// key.
///
/// # Panics
///
/// The function will panic if the cryptographic verification fails due to an
/// invalid signature, which is the expected behavior for signature verification
/// in Soroban contracts.
pub fn verify(
    e: &Env,
    signature_payload: &Bytes,
    public_key: &BytesN<32>,
    signature: &BytesN<64>,
) -> bool {
    e.crypto().ed25519_verify(public_key, signature_payload, signature);

    true
}

/// Returns the canonical byte representation of an Ed25519 public key.
///
/// Ed25519 public keys are 32-byte compressed Edwards curve points with a
/// single canonical encoding per key. The `BytesN<32>` type constraint
/// already enforces the correct length at deserialization, so this function
/// simply converts the fixed-size key to a `Bytes` value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `public_key` - The 32-byte Ed25519 public key.
pub fn canonicalize_key(e: &Env, public_key: &BytesN<32>) -> Bytes {
    Bytes::from_slice(e, &public_key.to_array())
}
