use soroban_sdk::{contracttype, panic_with_error, Env};

use crate::NonFungibleTokenError;

use super::NonFungibleSequential;

#[contracttype]
pub enum StorageKey {
    TokenIdCounter,
}

pub fn next_token_id<T: NonFungibleSequential>(e: &Env) -> u32 {
    e.storage().instance().get(&StorageKey::TokenIdCounter).unwrap_or(0)
}

// increase and return the previous
pub fn increment_token_id<T: NonFungibleSequential>(e: &Env, amount: u32) -> u32 {
    let current_id = T::next_token_id(e);
    let Some(next_id) = current_id.checked_add(amount) else {
        panic_with_error!(e, NonFungibleTokenError::MathOverflow);
    };
    e.storage().instance().set(&StorageKey::TokenIdCounter, &next_id);
    current_id
}
