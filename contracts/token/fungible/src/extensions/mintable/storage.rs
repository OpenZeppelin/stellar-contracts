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
/// # Events
///
/// * topics - `["mint", account: Address]`
/// * data - `[amount: i128]`
pub fn mint(e: &Env, account: &Address, amount: i128) {
    update(e, None, Some(account), amount);
    emit_mint(e, account, amount);
}
