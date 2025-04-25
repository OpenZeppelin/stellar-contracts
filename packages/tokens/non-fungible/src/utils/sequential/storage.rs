use soroban_sdk::{contracttype, panic_with_error, Env};

use crate::NonFungibleTokenError;

#[contracttype]
pub enum NFTSequentialStorageKey {
    TokenIdCounter,
}

/// Get the current token counter value to determine the next token_id.
/// The returned value is the next available token_id.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn next_token_id(e: &Env) -> u32 {
    e.storage().instance().get(&NFTSequentialStorageKey::TokenIdCounter).unwrap_or(0)
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
pub fn increment_token_id(e: &Env, amount: u32) -> u32 {
    let current_id = next_token_id(e);
    let Some(next_id) = current_id.checked_add(amount) else {
        panic_with_error!(e, NonFungibleTokenError::TokenIDsAreDepleted);
    };
    e.storage().instance().set(&NFTSequentialStorageKey::TokenIdCounter, &next_id);
    current_id
}
