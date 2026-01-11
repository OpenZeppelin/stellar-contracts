use cvlr::nondet::Nondet;
use cvlr_soroban::{nondet_bytes};
use soroban_sdk::Env;

use crate::crypto::keccak::Keccak256;
use crate::crypto::hasher::Hasher;

// my nondet for keccak, it does not use nondet() because it must
// receive an environment as a paramater - not sure.
pub fn nondet_keccak(e: &Env) -> Keccak256 {
    let is_some = bool::nondet();
    let mut new_keccak = Keccak256::new(e);
    if is_some {
        let state = nondet_bytes();
        new_keccak.update(state);
    }
    new_keccak
}
