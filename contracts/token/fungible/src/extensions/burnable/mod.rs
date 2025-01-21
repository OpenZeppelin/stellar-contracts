pub mod storage;
mod tests;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Burnable Trait for Fungible Token
///
/// The `Burnable` trait extends the `FungibleToken` trait to provide the
/// capability to burn tokens. This trait is designed to be used in conjunction
/// with the `FungibleToken` trait.
///
/// To fully comply with the SEP-41 specification one have to implement the
/// this `Burnable` trait along with the `[FungibleToken]` trait.
/// SEP-41 mandates support for token burning to be considered compliant.
///
/// Excluding the `burn` functionality from the `[FungibleToken]` trait
/// is a deliberate design choice to accommodate flexibility and customization
/// for various smart contract use cases.
#[contractclient(name = "FungibleBurnableTokenClient")]
pub trait FungibleBurnable {
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
    /// * [`crate::fungible::FungibleTokenError::InsufficientBalance`] - When
    ///   attempting to burn more tokens than `from` current balance.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::extensions::burnable::storage::burn()`]
    /// function the `storage` module when implementing this function.
    fn burn(e: &Env, from: &Address, amount: i128);

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
    /// * [`crate::fungible::FungibleTokenError::InsufficientBalance`] - When
    ///   attempting to burn more tokens than `from` current balance.A
    /// * [`crate::fungible::FungibleTokenError::InsufficientAllowance`] - When
    ///   attempting to burn more tokens than `from` allowance.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::extensions::burnable::storage::burn()`]
    /// function the `storage` module when implementing this function.
    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128);
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
