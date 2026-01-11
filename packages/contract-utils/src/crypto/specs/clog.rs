use cvlr::clog;
use crate::crypto::keccak::Keccak256;
use crate::crypto::hasher::Hasher;

pub fn clog_keccak(keccak: &Keccak256) {
    let state = keccak.get_state();
    if let Some(state) = state {
        clog!(cvlr_soroban::B(&state));
    }
}