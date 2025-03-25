use soroban_sdk::{contracttype, panic_with_error, Env};

use crate::{NonFungibleTokenError, TokenId};

#[contracttype]
pub enum StorageKey {
    TokenIdCounter,
}

pub fn next_token_id(e: &Env) -> TokenId {
    e.storage().instance().get(&StorageKey::TokenIdCounter).unwrap_or(0)
}

// increase and return the previous
pub fn increment_token_id(e: &Env, amount: TokenId) -> TokenId {
    let current_id = next_token_id(e);
    let Some(next_id) = current_id.checked_add(amount) else {
        panic_with_error!(e, NonFungibleTokenError::MathOverflow);
    };
    e.storage().instance().set(&StorageKey::TokenIdCounter, &next_id);
    current_id
}
