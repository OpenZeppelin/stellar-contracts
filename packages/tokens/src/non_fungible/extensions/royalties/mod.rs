mod storage;
use crate::non_fungible::NonFungibleToken;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{Address, Env};

/// Royalties Trait for Non-Fungible Token (ERC2981)
///
/// The `NonFungibleRoyalties` trait extends the `NonFungibleToken` trait to
/// provide the capability to set and query royalty information for tokens. This
/// trait is designed to be used in conjunction with the `NonFungibleToken`
/// trait.
///
/// This implementation is inspired by the ERC2981 standard for royalties, and
/// additionally, it allows:
/// - Get the royalty info for a token
/// - Set the global default royalty for the entire collection
/// - Set per-token royalties that override the global setting
/// - Remove per-token royalties to fall-back to the global royalty set for the
///   contract
///
/// `storage.rs` file of this module provides the `NonFungibleRoyalties` trait
/// implementation.
///
/// # Notes
///
/// In most marketplaces, royalty calculations are done in amounts of fungible
/// tokens. For example, if an NFT is sold for 10000 USDC and royalty is 10%,
/// 1000 USDC goes to the creator. To preserve the compatibility across
/// Non-Fungible and Fungible tokens, we are using `i128` instead of `u128` for
/// the `sale_price`, due to SEP-41.
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To avoid confusion, we do not provide the default
/// implementations here, but we are providing a macro that generates them.
///
/// ## Example
///
/// ```ignore
/// #[default_impl] // **IMPORTANT**: place this above `#[contractimpl]`
/// #[contractimpl]
/// impl NonFungibleRoyalties for MyContract {
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     /* and the macro will generate all the missing default implementations for you */
/// }
/// ```
pub trait NonFungibleRoyalties: NonFungibleToken {
    /// Sets the global default royalty information for the entire collection.
    /// This will be used for all tokens that don't have specific royalty
    /// information.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%,
    ///   10000 = 100%).
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::InvalidRoyaltyAmount`] -
    ///   If the royalty amount is higher than 10_000 (100%) basis points.
    ///
    /// # Events
    ///
    /// * topics - `["set_default_royalty", receiver: Address]`
    /// * data - `[basis_points: u32]`
    fn set_default_royalty(e: &Env, receiver: Address, basis_points: u32, operator: Address);

    /// Sets the royalty information for a specific token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%,
    ///   10000 = 100%).
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::InvalidRoyaltyAmount`] -
    ///   If the royalty amount is higher than 10_000 (100%) basis points.
    /// * [`crate::non_fungible::NonFungibleTokenError::NonExistentToken`] - If
    ///   the token does not exist.
    ///
    /// # Events
    ///
    /// * topics - `["set_token_royalty", receiver: Address]`
    /// * data - `[token_id: u32, basis_points: u32]`
    fn set_token_royalty(
        e: &Env,
        token_id: u32,
        receiver: Address,
        basis_points: u32,
        operator: Address,
    );

    /// Removes token-specific royalty information, allowing the token to fall
    /// back to the collection-wide default royalty settings.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::NonExistentToken`] - If
    ///   the token does not exist.
    ///
    /// # Events
    ///
    /// * topics - `["remove_token_royalty", token_id: u32]`
    /// * data - `[]`
    fn remove_token_royalty(e: &Env, token_id: u32, operator: Address);

    /// Returns `(Address, i128)` - A tuple containing the receiver address and
    /// the royalty amount.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `sale_price` - The sale price for which royalties are being
    ///   calculated.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::NonExistentToken`] - If
    ///   the token does not exist.
    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128);
}

// ################## EVENTS ##################

/// Event emitted when default royalty is set.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetDefaultRoyalty {
    #[topic]
    pub receiver: Address,
    pub basis_points: u32,
}

/// Emits an event indicating that default royalty has been set.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `receiver` - The royalty receiver address.
/// * `basis_points` - The royalty basis points.
#[cfg(not(feature = "certora"))]
pub fn emit_set_default_royalty(e: &Env, receiver: &Address, basis_points: u32) {
    SetDefaultRoyalty { receiver: receiver.clone(), basis_points }.publish(e);
}

/// Event emitted when token royalty is set.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetTokenRoyalty {
    #[topic]
    pub receiver: Address,
    #[topic]
    pub token_id: u32,
    pub basis_points: u32,
}

/// Emits an event indicating that token royalty has been set.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `receiver` - The royalty receiver address.
/// * `token_id` - The token identifier.
/// * `basis_points` - The royalty basis points.
#[cfg(not(feature = "certora"))]
pub fn emit_set_token_royalty(e: &Env, receiver: &Address, token_id: u32, basis_points: u32) {
    SetTokenRoyalty { receiver: receiver.clone(), token_id, basis_points }.publish(e);
}

/// Event emitted when token royalty is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveTokenRoyalty {
    #[topic]
    pub token_id: u32,
}

/// Emits an event indicating that token royalty has been removed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `token_id` - The token identifier.
#[cfg(not(feature = "certora"))]
pub fn emit_remove_token_royalty(e: &Env, token_id: u32) {
    RemoveTokenRoyalty { token_id }.publish(e);
}
