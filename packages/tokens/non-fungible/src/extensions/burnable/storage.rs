use soroban_sdk::{Address, Env};

use crate::{
    extensions::burnable::emit_burn,
    storage::{check_spender_approval, update},
};

/// Destroys the `token_id` from `account`, ensuring ownership
/// checks, and emits a `burn` event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The account whose token is destroyed.
/// * `token_id` - The token to burn.
///
/// # Errors
///
/// * refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: u128]`
///
/// # Notes
///
/// Authorization for `from` is required.
pub fn burn(e: &Env, from: &Address, token_id: u128) {
    from.require_auth();
    update(e, Some(from), None, token_id);
    emit_burn(e, from, token_id);
}

/// Destroys the `token_id` from `account`, ensuring ownership
/// and approval checks, and emits a `burn` event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The account that is allowed to burn the token on behalf of the
///   owner.
/// * `from` - The account whose token is destroyed.
/// * `token_id` - The token to burn.
///
/// # Errors
///
/// * refer to [`check_spender_approval`] errors.
/// * refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: u128]`
///
/// # Notes
///
/// Authorization for `spender` is required.
pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u128) {
    spender.require_auth();
    check_spender_approval(e, spender, from, token_id);
    update(e, Some(from), None, token_id);
    emit_burn(e, from, token_id);
}
