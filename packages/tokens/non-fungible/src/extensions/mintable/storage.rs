use soroban_sdk::{panic_with_error, Address, Env};

use crate::{extensions::mintable::emit_mint, storage::update, NonFungibleTokenError};

const COUNTER: &str = "COUNTER";

/// Get the current token counter value to determine the next token_id.
/// The returned value is the next available token_id.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_counter(e: &Env) -> u32 {
    e.storage().instance().get(&COUNTER).unwrap_or(0)
}

/// Increment and return the next token ID.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`crate::NonFungibleTokenError::MathOverflow`] - When all the available
///   `token_id`s are consumed for this smart contract.
pub fn increment_counter(e: &Env) -> u32 {
    let current = get_counter(e);
    let next = current.checked_add(1).unwrap_or_else(|| {
        panic_with_error!(e, NonFungibleTokenError::MathOverflow);
    });
    e.storage().instance().set(&COUNTER, &next);

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
/// * data - `[token_id: u32]`
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
pub fn mint(e: &Env, to: &Address) -> u32 {
    let token_id = increment_counter(e);
    update(e, None, Some(to), token_id);
    emit_mint(e, to, token_id);

    token_id
}
