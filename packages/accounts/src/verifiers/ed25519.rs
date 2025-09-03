/// Contract for verifying Ed25519 digital signatures.
///
/// This module provides Ed25519 signature verification functionality for
/// Stellar smart contracts. Ed25519 is a high-performance public-key signature
/// system that provides strong security guarantees.
use soroban_sdk::{contracterror, panic_with_error, xdr::FromXdr, Bytes, BytesN, Env};

use crate::verifiers::utils::extract_from_bytes;

/// Error types for Ed25519 signature verification operations.
#[contracterror]
#[repr(u32)]
pub enum Ed25519Error {
    /// The provided key data is invalid or has incorrect length.
    KeyDataInvalid = 2100,
    /// The signature format is invalid or cannot be parsed from XDR.
    SignatureFormatInvalid = 2101,
}

/// Ed25519 signature data type.
///
/// Represents a 64-byte Ed25519 signature that can be verified against a
/// message and public key.
pub type Ed25519SigData = BytesN<64>;

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
/// * `key_data` - The public key data (must be exactly 32 bytes).
/// * `sig_data` - XDR-encoded signature data (64 bytes).
///
/// # Returns
///
/// Returns `true` if the signature is valid for the given payload and public
/// key.
///
/// # Errors
///
/// * [`Ed25519Error::KeyDataInvalid`] - When the key data is not exactly 32
///   bytes or is malformed.
/// * [`Ed25519Error::SignatureFormatInvalid`] - When the signature data cannot
///   be parsed from XDR format.
///
/// # Panics
///
/// The function will panic if the cryptographic verification fails due to an
/// invalid signature, which is the expected behavior for signature verification
/// in Soroban contracts.
pub fn verify(e: &Env, signature_payload: Bytes, key_data: Bytes, sig_data: Bytes) -> bool {
    let public_key: BytesN<32> = extract_from_bytes(e, &key_data, 0..32)
        .unwrap_or_else(|| panic_with_error!(e, Ed25519Error::KeyDataInvalid));

    let signature: BytesN<64> = Ed25519SigData::from_xdr(e, &sig_data)
        .unwrap_or_else(|_| panic_with_error!(e, Ed25519Error::SignatureFormatInvalid));

    e.crypto().ed25519_verify(&public_key, &signature_payload, &signature);

    true
}
