// TODO: read all of this again (copied from Fungible)
mod storage;
pub use self::storage::{burn, burn_from};

mod test;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Burnable Trait for Non-Fungible Token
///
/// The `NonFungibleBurnable` trait extends the `NonFungibleToken` trait to provide
/// the capability to burn tokens. This trait is designed to be used in
/// conjunction with the `NonFungibleToken` trait.
///
/// To fully comply with the SEP-41 specification one have to implement the
/// this `NonFungibleBurnable` trait along with the `[NonFungibleToken]` trait.
/// SEP-41 mandates support for token burning to be considered compliant.
///
/// Excluding the `burn` functionality from the `[NonFungibleToken]` trait
/// is a deliberate design choice to accommodate flexibility and customization
/// for various smart contract use cases.
#[contractclient(name = "NonFungibleBurnableClient")]
pub trait NonFungibleBurnable {
    /// Destroys `amount` of tokens from `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose tokens are destroyed.
    /// * `token_id` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - When attempting
    ///   to burn a token that does not exist.
    /// * [`NonFungibleTokenError::IncorrectOwner`] - When trying to burn
    ///   a token that is not owned by the caller.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u128]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::burnable::burn()`] when implementing this
    /// function.
    fn burn(e: &Env, from: Address, token_id: u128);

    /// Destroys `amount` of tokens from `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to
    ///    burn tokens on behalf of the owner.
    /// * `from` - The account whose tokens are destroyed.
    /// * `token_id` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - When attempting
    ///   to burn a token that does not exist.
    /// * [`NonFungibleTokenError::IncorrectOwner`] - When trying to burn
    ///   a token that is not owned by the caller.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u128]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::burnable::burn_from()`] when implementing this
    /// function.
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
