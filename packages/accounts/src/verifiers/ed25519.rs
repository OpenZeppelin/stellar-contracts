use soroban_sdk::{contract, contractimpl, Bytes, BytesN, Env};

use crate::verifiers::{utils::extract_from_bytes, Verifier};

#[contract]
pub struct Ed2559VerifierContract;

#[contractimpl]
impl Verifier for Ed2559VerifierContract {
    fn verify(e: &Env, hash: Bytes, sig_data: Bytes) -> bool {
        let public_key: BytesN<32> = extract_from_bytes(e, &sig_data, 0..32);
        let signature: BytesN<64> = extract_from_bytes(e, &sig_data, 32..96);

        e.crypto().ed25519_verify(&public_key, &hash, &signature);

        true
    }
}
