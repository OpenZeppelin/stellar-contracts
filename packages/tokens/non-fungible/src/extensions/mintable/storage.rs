use soroban_sdk::{Address, Env};

use crate::{extensions::mintable::emit_mint, sequential::increment_token_id, Base, TokenId};

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
/// * refer to [`increment_token_id`] errors.
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
/// This function utilizes [`increment_token_id()`] to keep determine the next
/// `token_id`, but it does NOT check if the provided `token_id` is already in
/// use. If the developer has other means of minting tokens and generating
/// `token_id`s, they should ensure that the token_id is unique and not already
/// in use.
pub fn sequential_mint(e: &Env, to: &Address) -> TokenId {
    let token_id = increment_token_id(e, 1);
    Base::update(e, None, Some(to), token_id);
    emit_mint(e, to, token_id);

    token_id
}
