use soroban_sdk::{Bytes, BytesN};

use crate::verifiers::Verifier;

// a verifier with verify function that reads from a ghost mapping.
pub struct Verifier1;
use crate::smart_account::specs::ghosts::GhostMap;
pub static mut VERIFIER1_VERIFY_RESULT_MAP: GhostMap<(Bytes, BytesN<32>, BytesN<64>), bool> = GhostMap::UnInit;
impl Verifier for Verifier1 {
    type KeyData = BytesN<32>;
    type SigData = BytesN<64>;

    fn verify(
        e: &soroban_sdk::Env,
        hash: soroban_sdk::Bytes,
        key_data: Self::KeyData,
        sig_data: Self::SigData,
    ) -> bool {
        unsafe {
            VERIFIER1_VERIFY_RESULT_MAP.get(&(hash, key_data, sig_data))
        }
    }
}
