mod storage;
use crate::NonFungibleToken;

mod test;

use soroban_sdk::{Address, Env};

/// Maximum allowed royalty percentage (5000 basis points = 50%)
pub const MAX_ROYALTY_BASIS_POINTS: u32 = 5000;

/// Royalties Trait for Non-Fungible Token (ERC2981)
///
/// The `NonFungibleRoyalties` trait extends the `NonFungibleToken` trait to
/// provide the capability to set and query royalty information for tokens. This trait
/// is designed to be used in conjunction with the `NonFungibleToken` trait.
///
/// This implementation follows the ERC2981 standard for royalties, allowing:
/// - Setting global royalties for the entire collection
/// - Setting per-token royalties that override the global setting
/// - Enforcing a maximum royalty percentage to prevent scams
/// - Making royalties immutable after minting
///
/// `storage.rs` file of this module provides the `NonFungibleRoyalties` trait
/// implementation for the `Base` contract type. For other contract types (eg.
/// `Enumerable`, `Consecutive`), the overrides of the `NonFungibleRoyalties`
/// trait methods can be found in their respective `storage.rs` file.
///
/// # Notes
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To not confuse the developers, we did not provide
/// the default implementations here, but we are providing a macro to generate
/// the default implementations for you.
///
/// When implementing [`NonFungibleRoyalties`] trait for your Smart Contract,
/// you can follow the below example:
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
    /// This will be used for all tokens that don't have specific royalty information.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 = 100%).
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::RoyaltyTooHigh`] - If the royalty percentage exceeds
    ///   the maximum allowed value.
    fn set_default_royalty(e: &Env, receiver: Address, basis_points: u32);

    /// Sets the royalty information for a specific token.
    /// This must be called during minting, as royalties are immutable after minting.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 = 100%).
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - If the token does not exist.
    /// * [`crate::NonFungibleTokenError::RoyaltyTooHigh`] - If the royalty percentage exceeds
    ///   the maximum allowed value.
    /// * [`crate::NonFungibleTokenError::RoyaltyAlreadySet`] - If attempting to set royalties
    ///   for a token that already has royalty information.
    fn set_token_royalty(e: &Env, token_id: u32, receiver: Address, basis_points: u32);

    /// Returns the royalty information for a token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `sale_price` - The sale price for which royalties are being calculated.
    ///
    /// # Returns
    ///
    /// * `(Address, u32)` - A tuple containing the receiver address and the royalty amount.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - If the token does not exist.
    fn royalty_info(e: &Env, token_id: u32, sale_price: u32) -> (Address, u32);
}
