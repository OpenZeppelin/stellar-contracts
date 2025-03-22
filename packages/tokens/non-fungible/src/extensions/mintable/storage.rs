use soroban_sdk::{Address, Env};

use crate::{extensions::mintable::emit_mint, sequential::NonFungibleSequential, NonFungibleToken};

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
///
/// This function utilizes [`increment_counter()`] to keep determine the next
/// `token_id`, but it does NOT check if the provided `token_id` is already in
/// use. If the developer has other means of minting tokens and generating
/// `token_id`s, they should ensure that the token_id is unique and not already
/// in use.
pub fn mint<T: NonFungibleToken>(e: &Env, to: &Address, token_id: u32) -> u32 {
    crate::storage::update::<T>(e, None, Some(to), token_id);
    emit_mint(e, to, token_id);

    token_id
}

pub fn sequential_mint<T: NonFungibleToken + NonFungibleSequential>(e: &Env, to: &Address) -> u32 {
    let token_id = T::increment_token_id(e, 1);
    mint::<T>(e, to, token_id)
}
