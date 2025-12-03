//! # WebAuthn Verifier Contract
//!
//! A reusable verifier contract for WebAuthn (passkey) signature verification.
//! This contract can be deployed once and used by multiple smart accounts
//! across the network for delegated signature verification. Provides
//! cryptographic verification for WebAuthn signatures against message hashes
//! and public keys.
//!
//! Unlike simpler signature schemes, WebAuthn signature data is a complex
//! structure containing authenticator data, client data JSON, and the signature
//! itself. The `sig_data` parameter should be XDR-encoded `WebAuthnSigData` to
//! ensure proper serialization and deserialization.
//!
//! The `key_data` parameter is expected to contain the 65-byte uncompressed
//! secp256r1 public key followed by the credential ID bytes (if any).
use soroban_sdk::{contract, contractimpl, xdr::FromXdr, Bytes, BytesN, Env};
use stellar_accounts::verifiers::{
    utils::extract_from_bytes,
    webauthn::{self, WebAuthnSigData},
    Verifier,
};

#[contract]
pub struct WebauthnVerifierContract;

#[contractimpl]
impl Verifier for WebauthnVerifierContract {
    type KeyData = Bytes;
    type SigData = Bytes;

    /// Verify a WebAuthn signature against a message and public key.
    ///
    /// # Arguments
    ///
    /// * `signature_payload` - The message hash that was signed
    /// * `key_data` - Bytes containing:
    ///   - 65-byte secp256r1 public key (uncompressed format)
    ///   - Variable length credential ID (used on the client side)
    /// * `sig_data` - XDR-encoded `WebAuthnSigData` structure containing:
    ///   - Authenticator data
    ///   - Client data JSON
    ///   - Signature components
    ///
    /// # Returns
    ///
    /// * `true` if the signature is valid
    /// * `false` otherwise
    fn verify(e: &Env, signature_payload: Bytes, key_data: Bytes, sig_data: Bytes) -> bool {
        let sig_struct =
            WebAuthnSigData::from_xdr(e, &sig_data).expect("WebAuthnSigData with correct format");

        let pub_key: BytesN<65> =
            extract_from_bytes(e, &key_data, 0..65).expect("65-byte public key to be extracted");

        webauthn::verify(e, &signature_payload, &pub_key, &sig_struct)
    }
}
