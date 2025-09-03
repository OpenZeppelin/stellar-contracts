use soroban_sdk::{contract, contractimpl, Bytes, Env};
use stellar_accounts::verifiers::ed25519;

#[contract]
pub struct Ed25519VerifierContract;

#[contractimpl]
impl Ed25519VerifierContract {
    /// Verify an Ed25519 signature against a message and public key
    pub fn verify(e: Env, signature_payload: Bytes, key_data: Bytes, sig_data: Bytes) -> bool {
        ed25519::verify(&e, signature_payload, key_data, sig_data)
    }
}
