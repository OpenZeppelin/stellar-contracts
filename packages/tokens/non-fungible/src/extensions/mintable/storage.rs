use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{extensions::mintable::emit_mint, storage::update, NonFungibleTokenError, TokenId};

#[contracttype]
pub enum DataKey {
    TokenIdCounter,
}

/// Get the current token counter value to determine the next token_id.
/// The returned value is the next available token_id.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn next_token_id(e: &Env) -> TokenId {
    e.storage().instance().get(&DataKey::TokenIdCounter).unwrap_or(0)
}

/// Return the next free token ID, then increment the counter.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`crate::NonFungibleTokenError::TokenIDsAreDepleted`] - When all the
///   available `token_id`s are consumed for this smart contract.
pub fn increment_token_id(e: &Env) -> TokenId {
    let current = next_token_id(e);
    let next = current.checked_add(1).unwrap_or_else(|| {
        panic_with_error!(e, NonFungibleTokenError::TokenIDsAreDepleted);
    });
    e.storage().instance().set(&DataKey::TokenIdCounter, &next);

    current
}

/// Creates a token with the next available `token_id` and assigns it to `to`.
/// Returns the `token_id` for the newly minted token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new token.
///
/// # Errors
///
/// * refer to [`increment_counter`] errors.
/// * refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[token_id: TokenId]`
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate access
/// controls to ensure that only authorized accounts can execute minting
/// operations. Failure to implement proper authorization could lead to
/// security vulnerabilities and unauthorized token creation.
///
/// You probably want to do something like this (pseudo-code):
///
/// ```ignore
/// let admin = read_administrator(e);
/// admin.require_auth();
/// ```
///
/// This function utilizes [`increment_counter()`] to keep determine the next
/// `token_id`, but it does NOT check if the provided `token_id` is already in
/// use. If the developer has other means of minting tokens and generating
/// `token_id`s, they should ensure that the token_id is unique and not already
/// in use.
pub fn sequential_mint(e: &Env, to: &Address) -> TokenId {
    let token_id = increment_token_id(e);
    update(e, None, Some(to), token_id);
    emit_mint(e, to, token_id);

    token_id
}
