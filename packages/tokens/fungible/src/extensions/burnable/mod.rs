mod storage;

mod test;

use soroban_sdk::{symbol_short, Env};


/// Burnable Trait for Fungible Token
///
/// The `FungibleBurnable` trait extends the `FungibleToken` trait to provide
/// the capability to burn tokens. This trait is designed to be used in
/// conjunction with the `FungibleToken` trait.
///
/// To fully comply with the SEP-41 specification one have to implement the
/// this `FungibleBurnable` trait along with the `[FungibleToken]` trait.
/// SEP-41 mandates support for token burning to be considered compliant.
///
/// Excluding the `burn` functionality from the `[FungibleToken]` trait
/// is a deliberate design choice to accommodate flexibility and customization
/// for various smart contract use cases.
#[soroban_sdk::contracttrait(default = Base, is_extension = true)]
pub trait FungibleBurnable {
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
    /// * [`crate::FungibleTokenError::InsufficientBalance`] - When attempting
    ///   to burn more tokens than `from` current balance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    fn burn(e: &Env, from: &soroban_sdk::Address, amount: i128);

    /// Destroys `amount` of tokens from `from`. Updates the total
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
    ///   to burn more tokens than `from` current balance.
    /// * [`crate::FungibleTokenError::InsufficientAllowance`] - When attempting
    ///   to burn more tokens than `from` allowance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    fn burn_from(e: &Env, spender: &soroban_sdk::Address, from: &soroban_sdk::Address, amount: i128);
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
pub fn emit_burn(e: &Env, from: &soroban_sdk::Address, amount: i128) {
    let topics = (symbol_short!("burn"), from);
    e.events().publish(topics, amount)
}
