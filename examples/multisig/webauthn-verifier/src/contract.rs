//! # WebAuthn Verifier Contract
//!
//! A reusable verifier contract for WebAuthn (passkey) signature verification.
//! This contract can be deployed once and used by multiple smart accounts across
//! the network for delegated signature verification. Provides cryptographic
//! verification for WebAuthn signatures against message hashes and public keys.
//!
//! Unlike simpler signature schemes, WebAuthn signature data is a complex
//! structure containing authenticator data, client data JSON, and the signature
//! itself. The `sig_data` parameter should be XDR-encoded `WebAuthnSigData` to
//! ensure proper serialization and deserialization.
use soroban_sdk::{contract, contractimpl, xdr::FromXdr, Bytes, BytesN, Env};
use stellar_accounts::verifiers::{
    webauthn::{self, WebAuthnSigData},
    Verifier,
};

#[contract]
pub struct WebauthnVerifierContract;

#[contractimpl]
impl Verifier for WebauthnVerifierContract {
    type KeyData = BytesN<65>;
    type SigData = Bytes;

    /// Verify a WebAuthn signature against a message and public key.
    ///
    /// The `sig_data` parameter must be an XDR-encoded `WebAuthnSigData`
    /// structure containing the authenticator data, client data JSON, and
    /// signature components required for WebAuthn verification.
    fn verify(e: &Env, signature_payload: Bytes, key_data: BytesN<65>, sig_data: Bytes) -> bool {
        let sig_struct =
            WebAuthnSigData::from_xdr(e, &sig_data).expect("WebAuthnSigData wrong format");
        webauthn::verify(e, &signature_payload, &key_data, &sig_struct)
    }
}
