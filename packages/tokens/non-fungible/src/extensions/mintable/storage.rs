use soroban_sdk::{Address, Env};

use crate::{extensions::mintable::emit_mint, storage::update};

/// Creates a token with `token_id` and assigns it to `to`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new token.
/// * `token_id` - Token id as a number.
///
/// # Errors
///
/// refer to [`update`] errors.
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
pub fn mint(e: &Env, to: &Address, token_id: u32) {
    update(e, None, Some(to), token_id);
    emit_mint(e, to, token_id);
}
