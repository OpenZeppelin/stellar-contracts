use soroban_sdk::{Address, Env};

use crate::{extensions::mintable::emit_mint, storage::update};

/// Creates `amount` of tokens and assigns them to `account`. Updates
/// the total supply accordingly.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
///
/// # Errors
///
/// refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["mint", account: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// IMPORTANT: This function lacks authorization controls. It is the
/// responsibility of the implementer to establish appropriate access
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
pub fn mint(e: &Env, account: &Address, amount: i128) {
    update(e, None, Some(account), amount);
    emit_mint(e, account, amount);
}
