use soroban_sdk::{panic_with_error, Bytes, BytesN, Env};

use crate::{error::CryptoError, hasher::Hasher};

/// Struct to store bytes that will be consumed by the keccak256 [`Hasher`]
/// implementation.
pub struct Keccak256 {
    state: Option<Bytes>,
    env: Env,
}

impl Hasher for Keccak256 {
    type Output = BytesN<32>;

    fn new(e: &Env) -> Self {
        Keccak256 { state: None, env: e.clone() }
    }

    fn update(&mut self, input: Bytes) {
        match &mut self.state {
            None => self.state = Some(input),
            Some(state) => state.append(&input),
        }
    }

    fn finalize(self) -> Self::Output {
        let data = self
            .state
            .unwrap_or_else(|| panic_with_error!(&self.env, CryptoError::HasherEmptyState));
        self.env.crypto().keccak256(&data).to_bytes()
    }
}
