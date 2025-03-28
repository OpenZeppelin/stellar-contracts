use soroban_sdk::{contracttype, panic_with_error, Env};

use crate::{NonFungibleTokenError, TokenId};

#[contracttype]
pub enum StorageKey {
    TokenIdCounter,
}

/// Get the current token counter value to determine the next token_id.
/// The returned value is the next available token_id.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn next_token_id(e: &Env) -> TokenId {
    e.storage().instance().get(&StorageKey::TokenIdCounter).unwrap_or(0)
}

/// Return the next free token ID, then increment the counter.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The number by which the counter is incremented.
///
/// # Errors
///
/// * [`crate::NonFungibleTokenError::TokenIDsAreDepleted`] - When all the
///   available `token_id`s are consumed for this smart contract.
pub fn increment_token_id(e: &Env, amount: TokenId) -> TokenId {
    let current_id = next_token_id(e);
    let Some(next_id) = current_id.checked_add(amount) else {
        panic_with_error!(e, NonFungibleTokenError::MathOverflow);
    };
    e.storage().instance().set(&StorageKey::TokenIdCounter, &next_id);
    current_id
}
