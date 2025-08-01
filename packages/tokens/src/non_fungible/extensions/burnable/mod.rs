mod storage;

mod test;

use soroban_sdk::{contracttrait, symbol_short, Address, Env};

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
/// implementation for the `NFTBase` contract type. For other contract types
/// (eg. `Enumerable`, `Consecutive`), the overrides of the
/// `NonFungibleBurnable` trait methods can be found in their respective
/// `storage.rs` file.
///
/// This approach lets us to implement the `NonFungibleBurnable` trait in a very
/// flexible way based on the `Impl` associated type from
/// `NonFungibleToken`:
///
/// ```ignore
/// #[contracttrait]
/// impl NonFungibleBurnable for ExampleContract {}
/// // Uses `NFTBase` as the default
///
/// #[contracttrait]
/// impl NonFungibleBurnable for ExampleContract {
///     type Impl = Enumerable;
/// }
///
///
/// ```
///
/// # Notes
///
/// `#[contracttrait]` macro provides a default implementation and generates a
/// `#[contractimpl]` with all the trait's methods forwarding them to `MyContract`.
///
/// When implementing [`NonFungibleBurnable`] trait for your Smart Contract,
/// you can follow the below example:
///
/// ## Example
///
/// ```ignore
/// #[contracttrait]
/// impl NonFungibleBurnable for MyContract {
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     // Can also provide a different default implementation
///     type Impl = Enumerable; // Or Consectutive
/// }
/// ```
#[contracttrait(default = NFTBase)]
pub trait NonFungibleBurnable {
    /// Destroys the token with `token_id` from `from`.
    ///
    /// # Arguments
    ///
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
    fn burn(e: &Env, from: &soroban_sdk::Address, token_id: u32);

    /// Destroys the token with `token_id` from `from`, by using `spender`s
    /// approval.
    ///
    /// # Arguments
    ///
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
    fn burn_from(
        e: &Env,
        spender: &soroban_sdk::Address,
        from: &soroban_sdk::Address,
        token_id: u32,
    );
}

// ################## EVENTS ##################

/// Emits an event indicating a burn of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `token_id` - The token ID of the burned token.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: u32]`
pub fn emit_burn(e: &Env, from: &Address, token_id: u32) {
    let topics = (symbol_short!("burn"), from);
    e.events().publish(topics, token_id)
}
