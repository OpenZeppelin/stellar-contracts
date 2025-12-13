use soroban_sdk::BytesN;

use crate::verifiers::Verifier;

pub struct SimpleVerifier;

// Taken from the documentation in `verifiers/mod.rs`

impl Verifier for SimpleVerifier {
    type KeyData = BytesN<32>;
    type SigData = BytesN<64>;

    // NOTE: maybe needs a fancier implementation
    fn verify(
        e: &soroban_sdk::Env,
        hash: soroban_sdk::Bytes,
        key_data: Self::KeyData,
        sig_data: Self::SigData,
    ) -> bool {
        true
    }
}
