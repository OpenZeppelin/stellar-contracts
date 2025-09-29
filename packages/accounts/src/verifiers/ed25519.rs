/// Contract for verifying Ed25519 digital signatures.
///
/// This module provides Ed25519 signature verification functionality for
/// Stellar smart contracts. Ed25519 is a high-performance public-key signature
/// system that provides strong security guarantees.
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
