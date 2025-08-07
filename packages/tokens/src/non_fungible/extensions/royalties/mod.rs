mod storage;

mod test;

use soroban_sdk::{contracttrait, Address, Env, Symbol};

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
/// `#[contracttrait]` macro provides a default implementation and generates a
/// `#[contractimpl]` with all the trait's methods forwarding them to
/// `MyContract`.
///
/// ## Example
///
/// ```ignore
/// #[contracttrait]
/// impl NonFungibleRoyalties for MyContract {
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
/// }
/// ```
#[contracttrait(add_impl_type = true)]
pub trait NonFungibleRoyalties {
    /// Sets the global default royalty information for the entire collection.
    /// This will be used for all tokens that don't have specific royalty
    /// information.
    ///
    /// # Arguments
    ///
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
    fn set_default_royalty(
        e: &Env,
        receiver: &soroban_sdk::Address,
        basis_points: u32,
        operator: &soroban_sdk::Address,
    );

    /// Sets the royalty information for a specific token.
    ///
    /// # Arguments
    ///
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
        receiver: &soroban_sdk::Address,
        basis_points: u32,
        operator: &soroban_sdk::Address,
    );

    /// Removes token-specific royalty information, allowing the token to fall
    /// back to the collection-wide default royalty settings.
    ///
    /// # Arguments
    ///
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
    fn remove_token_royalty(e: &Env, token_id: u32, operator: &soroban_sdk::Address);

    /// Returns `(Address, i128)` - A tuple containing the receiver address and
    /// the royalty amount.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The identifier of the token.
    /// * `sale_price` - The sale price for which royalties are being
    ///   calculated.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::NonExistentToken`] - If the token does
    ///   not exist.
    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (soroban_sdk::Address, i128);
}

// ################## EVENTS ##################

/// Emits an event indicating that default royalty has been set.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `receiver` - The address that will receive royalty payments.
/// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 =
///   100%).
///
/// # Events
///
/// * topics - `["set_default_royalty", receiver: Address]`
/// * data - `[basis_points: u32]`
pub fn emit_set_default_royalty(e: &Env, receiver: &Address, basis_points: u32) {
    let topics = (Symbol::new(e, "set_default_royalty"), receiver);
    e.events().publish(topics, basis_points);
}

/// Emits an event indicating that token royalty has been set.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `receiver` - The address that will receive royalty payments.
/// * `token_id` - The identifier of the token.
/// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 =
///   100%).
///
/// # Events
///
/// * topics - `["set_token_royalty", receiver: Address, token_id: u32]`
/// * data - `[basis_points: u32]`
pub fn emit_set_token_royalty(e: &Env, receiver: &Address, token_id: u32, basis_points: u32) {
    let topics = (Symbol::new(e, "set_token_royalty"), receiver, token_id);
    e.events().publish(topics, basis_points);
}

/// Emits an event indicating that token royalty has been removed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `token_id` - The identifier of the token.
///
/// # Events
///
/// * topics - `["remove_token_royalty", token_id: u32]`
/// * data - `[]`
pub fn emit_remove_token_royalty(e: &Env, token_id: u32) {
    let topics = (Symbol::new(e, "remove_token_royalty"), token_id);
    e.events().publish(topics, ());
}
