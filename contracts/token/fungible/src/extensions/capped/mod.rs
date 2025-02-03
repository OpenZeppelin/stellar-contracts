// mod storage;
// pub use self::storage::{burn, burn_from};

// mod test;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Capped Trait for Fungible Token
///
/// The `Capped` trait extends the `FungibleToken` trait to provide the
/// capability to set a limit for total supply.
#[contractclient(name = "FungibleBurnableClient")]
pub trait FungibleCapped {
    /// Destroys `amount` of tokens from `account`. Updates the total
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
    /// * [`crate::FungibleTokenError::InsufficientBalance`] - When attempting
    ///   to burn more tokens than `from` current balance.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::burnable::burn()`] when implementing this
    /// function.
    fn burn(e: &Env, from: Address, amount: i128);

    /// Destroys `amount` of tokens from `account`. Updates the total
    /// supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorized to burn the tokens.
    /// * `from` - The account whose tokens are destroyed.
    /// * `amount` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::FungibleTokenError::InsufficientBalance`] - When attempting
    ///   to burn more tokens than `from` current balance.A
    /// * [`crate::FungibleTokenError::InsufficientAllowance`] - When attempting
    ///   to burn more tokens than `from` allowance.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::burnable::burn_from()`] when implementing
    /// this function.
    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128);
}

// ################## EVENTS ##################

/// Emits an event indicating a burn of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `amount` - The amount of tokens to be burned.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[amount: i128]`
pub fn emit_burn(e: &Env, from: &Address, amount: i128) {
    let topics = (symbol_short!("burn"), from);
    e.events().publish(topics, amount)
}
