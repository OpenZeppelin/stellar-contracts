use soroban_sdk::{panic_with_error, Bytes, BytesN, Env};

use crate::{error::CryptoError, hasher::Hasher};

/// Struct to store bytes that will be consumed by the sha256 [`Hasher`]
/// implementation.
pub struct Sha256 {
    state: Option<Bytes>,
    env: Env,
}

impl Hasher for Sha256 {
    type Output = BytesN<32>;

    fn new(e: &Env) -> Self {
        Sha256 { state: None, env: e.clone() }
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
        self.env.crypto().sha256(&data).to_bytes()
    }
}
