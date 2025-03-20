mod storage;
pub use self::storage::mint;

mod test;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Mintable Trait for Non-Fungible Token
///
/// The `NonFungibleMintable` trait extends the `NonFungibleToken` trait to
/// provide the capability to mint tokens. This trait is designed to be used in
/// conjunction with the `NonFungibleToken` trait.
///
/// Excluding the `mint` functionality from the
/// [`crate::non_fungible::NonFungibleToken`] trait is a deliberate design
/// choice to accommodate flexibility and customization for various smart
/// contract use cases.
#[contractclient(name = "NonFungibleMintableClient")]
pub trait NonFungibleMintable {
    /// Creates a token with the next available `token_id` and assigns it to
    /// `to`. Returns the `token_id` for the newly minted token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new token.
    ///
    /// # Errors
    ///
    /// * [`crate::NonFungibleTokenError::TokenIDsAreDepleted`] - When all the
    ///   available `token_id`s are consumed for this smart contract.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::mintable::mint()`] when implementing this
    /// function.
    ///
    /// # Security Warning
    ///
    /// IMPORTANT: The base implementation of mint() intentionally lacks
    /// authorization controls. You MUST implement proper authorization in
    /// your contract. For example:
    ///
    /// ```rust
    /// fn mint(&self, e: &Env, to: Address, token_id: u32) {
    ///     // 1. Verify admin has minting privileges (optional)
    ///     let admin = e.storage().instance().get(&ADMIN_KEY).unwrap();
    ///     admin.require_auth();
    ///
    ///     // 2. Only then call the actual mint function
    ///     crate::mintable::mint(e, &to, token_id);
    /// }
    /// ```
    ///
    /// Failing to add proper authorization could allow anyone to mint tokens!
    fn mint(e: &Env, to: Address) -> u32;
}

// ################## EVENTS ##################

/// Emits an event indicating a mint of a token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new token.
/// * `token_id` - Token id as a number.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[token_id: u32]`
pub fn emit_mint(e: &Env, to: &Address, token_id: u32) {
    let topics = (symbol_short!("mint"), to);
    e.events().publish(topics, token_id)
}
