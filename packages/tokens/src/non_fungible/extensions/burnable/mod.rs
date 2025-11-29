mod storage;
use crate::non_fungible::NonFungibleToken;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{Address, Env};

/// Burnable Trait for Non-Fungible Token
///
/// The `NonFungibleBurnable` trait extends the `NonFungibleToken` trait to
/// provide the capability to burn tokens. This trait is designed to be used in
/// conjunction with the `NonFungibleToken` trait.
///
/// Excluding the `burn` functionality from the
/// [`crate::non_fungible::NonFungibleToken`] trait is a deliberate design
/// choice to accommodate flexibility and customization for various smart
/// contract use cases.
///
/// `storage.rs` file of this module provides the `NonFungibelBurnable` trait
/// implementation for the `Base` contract type. For other contract types (eg.
/// `Enumerable`, `Consecutive`), the overrides of the `NonFungibleBurnable`
/// trait methods can be found in their respective `storage.rs` file.
///
/// This approach lets us to implement the `NonFungibleBurnable` trait in a very
/// flexible way based on the `ContractType` associated type from
/// `NonFungibleToken`:
///
/// ```ignore
/// impl NonFungibleBurnable for ExampleContract {
///     fn burn(e: &Env, from: Address, token_id: u32) {
///         Self::ContractType::burn(e, &from, token_id);
///     }
///
///     fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
///         Self::ContractType::burn_from(e, &spender, &from, token_id);
///     }
/// }
/// ```
///
/// # Notes
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To not confuse the developers, we did not provide
/// the default implementations here, but we are providing a macro to generate
/// the default implementations for you.
///
/// When implementing [`NonFungibleBurnable`] trait for your Smart Contract,
/// you can follow the below example:
///
/// ```ignore
/// #[default_impl] // **IMPORTANT**: place this above `#[contractimpl]`
/// #[contractimpl]
/// impl NonFungibleBurnable for MyContract {
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     /* and the macro will generate all the missing default implementations for you */
/// }
/// ```
pub trait NonFungibleBurnable: NonFungibleToken {
    /// Destroys the token with `token_id` from `from`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::NonExistentToken`] -
    ///   When attempting to burn a token that does not exist.
    /// * [`crate::non_fungible::NonFungibleTokenError::IncorrectOwner`] - If
    ///   the current owner (before calling this function) is not `from`.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    fn burn(e: &Env, from: Address, token_id: u32);

    /// Destroys the token with `token_id` from `from`, by using `spender`s
    /// approval.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to burn the token on behalf of
    ///   the owner.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::NonExistentToken`] -
    ///   When attempting to burn a token that does not exist.
    /// * [`crate::non_fungible::NonFungibleTokenError::IncorrectOwner`] - If
    ///   the current owner (before calling this function) is not `from`.
    /// * [`crate::non_fungible::NonFungibleTokenError::InsufficientApproval`] -
    ///   If the spender does not have a valid approval.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32);
}

// ################## EVENTS ##################

/// Event emitted when a token is burned.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Burn {
    #[topic]
    pub from: Address,
    pub token_id: u32,
}

/// Emits an event for a burn of a token from `from`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `from` - The sender address.
/// * `token_id` - The token identifier.
#[cfg(not(feature = "certora"))]
pub fn emit_burn(e: &Env, from: &Address, token_id: u32) {
    Burn { from: from.clone(), token_id }.publish(e);
}
