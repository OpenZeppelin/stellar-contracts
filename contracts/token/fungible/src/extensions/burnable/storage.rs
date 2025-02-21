use soroban_sdk::{Address, Env};

use crate::{
    extensions::burnable::emit_burn,
    storage::{spend_allowance, update},
};

/// Destroys `amount` of tokens from `from`. Updates the total
/// supply accordingly.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The account whose tokens are destroyed.
/// * `amount` - The amount of tokens to burn.
///
/// # Errors
///
/// * [`crate::FungibleTokenError::InsufficientBalance`] - When attempting to
///   burn more tokens than `from` current balance.
/// * [`crate::FungibleTokenError::LessThanZero`] - When `amount < 0`.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// Authorization for `from` is required.
pub fn burn(e: &Env, from: &Address, amount: i128) {
    from.require_auth();
    update(e, Some(from), None, amount);
    emit_burn(e, from, amount);
}

/// Destroys `amount` of tokens from `from` using the allowance mechanism.
/// `amount` is then deducted from `spender` allowance.
/// Updates the total supply accordingly.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The address authorizing the transfer, and having its
///   allowance.
/// * `from` - The account whose tokens are destroyed.
/// * `amount` - The amount of tokens to burn.
///
/// # Errors
///
/// * [`crate::FungibleTokenError::InsufficientBalance`] - When attempting to
///   burn more tokens than `from` current balance.
/// * [`crate::FungibleTokenError::InsufficientAllowance`] - When attempting to
///   burn more tokens than `spender`s current allowance.
/// * [`crate::FungibleTokenError::LessThanZero`] - When `amount < 0`.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// Authorization for `spender` is required.
pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
    spender.require_auth();
    spend_allowance(e, from, spender, amount);
    update(e, Some(from), None, amount);
    emit_burn(e, from, amount);
}
