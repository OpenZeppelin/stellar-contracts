//! # Ed25519 Verifier Contract
//!
//! A reusable verifier contract for Ed25519 signature verification. This
//! contract can be deployed once and used by multiple smart accounts across the
//! network for delegated signature verification. Provides cryptographic
//! verification for Ed25519 signatures against message hashes and public keys.
use soroban_sdk::{contract, contractimpl, Bytes, BytesN, Env};
use stellar_accounts::verifiers::{ed25519, Verifier};

#[contract]
pub struct Ed25519VerifierContract;

#[contractimpl]
impl Verifier for Ed25519VerifierContract {
    type KeyData = BytesN<32>;
    type SigData = BytesN<64>;

    /// Verify an Ed25519 signature against a message and public key.
    ///
    /// # Arguments
    ///
    /// * `signature_payload` - The message hash that was signed
    /// * `key_data` - The 32-byte Ed25519 public key
    /// * `sig_data` - The 64-byte Ed25519 signature
    ///
    /// # Returns
    ///
    /// * `true` if the signature is valid
    /// * `false` otherwise
    fn verify(
        e: &Env,
        signature_payload: Bytes,
        key_data: BytesN<32>,
        sig_data: BytesN<64>,
    ) -> bool {
        ed25519::verify(e, &signature_payload, &key_data, &sig_data)
    }
}
