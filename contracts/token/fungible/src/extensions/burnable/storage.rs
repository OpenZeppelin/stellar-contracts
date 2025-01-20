use crate::extensions::burnable::emit_burn;

/// Destroys a `value` amount of tokens from `account`. Updates the total
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
/// * [`crate::fungible::FungibleTokenError::InsufficientBalance`] -
/// When attempting to burn more tokens than `from` current balance.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[value: i128]`
pub fn burn(e: &Env, from: &Address, amount: i128) {
    update(e, Some(account), None, value);
    emit_burn(e, from, value);
}

/// Destroys `amount` of tokens from `account` using the allowance mechanism.
/// `amount`is then deducted from `spender` allowance.
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
/// * [`crate::fungible::FungibleTokenError::InsufficientBalance`] -
/// When attempting to burn more tokens than `from` current balance.
/// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
/// burn more tokens than `spender`s current allowance.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[value: i128]`
///
/// # Notes
///
/// Authorization for `spender` is required.
pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
    spender.require_auth();
    spend_allowance(e, from, spender, value);
    update(e, Some(account), None, value);
    emit_burn(e, from, value);
}
