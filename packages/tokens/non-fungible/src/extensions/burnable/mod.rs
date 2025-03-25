mod storage;
pub use self::storage::{burn, burn_from};
use crate::{Base, NonFungibleToken};

mod test;

use soroban_sdk::{symbol_short, Address, Env};

/// Burnable Trait for Non-Fungible Token
///
/// The `NonFungibleBurnable` trait extends the `NonFungibleToken` trait to
/// provide the capability to burn tokens. This trait is designed to be used in
/// conjunction with the `NonFungibleToken` trait.
///
/// Excluding the `burn` functionality from the `[NonFungibleToken]` trait
/// is a deliberate design choice to accommodate flexibility and customization
/// for various smart contract use cases.
pub trait NonFungibleBurnable: NonFungibleToken<ContractType = Base> {
    /// Destroys the `token_id` from `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The token to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - When attempting
    ///   to burn a token that does not exist.
    /// * [`crate::NonFungibleTokenError::IncorrectOwner`] - If the current
    ///   owner (before calling this function) is not `from`.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    fn burn(e: &Env, from: Address, token_id: u32) {
        crate::burnable::burn(e, &from, token_id);
    }

    /// Destroys the `token_id` from `account`, by using `spender`s approval.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to burn the token on behalf of
    ///   the owner.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The token to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - When attempting
    ///   to burn a token that does not exist.
    /// * [`crate::NonFungibleTokenError::IncorrectOwner`] - If the current
    ///   owner (before calling this function) is not `from`.
    /// * [`crate::NonFungibleTokenError::InsufficientApproval`] - If the
    ///   spender does not have a valid approval.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        crate::burnable::burn_from(e, &spender, &from, token_id);
    }
}

// ################## EVENTS ##################

/// Emits an event indicating a burn of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `token_id` - The burned token.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: u32]`
pub fn emit_burn(e: &Env, from: &Address, token_id: u32) {
    let topics = (symbol_short!("burn"), from);
    e.events().publish(topics, token_id)
}
