use soroban_sdk::{Address, Env};

use crate::{extensions::mintable::emit_mint, storage::update};

/// Creates `amount` of tokens and assigns them to `to`. Updates
/// the total supply accordingly.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
///
/// # Errors
///
/// refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[amount: i128]`
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
pub fn mint(e: &Env, to: &Address, amount: i128) {
    update(e, None, Some(to), amount);
    emit_mint(e, to, amount);
}
