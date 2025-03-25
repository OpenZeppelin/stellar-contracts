mod storage;
pub use self::storage::mint;

mod test;

use soroban_sdk::{symbol_short, Address, Env};

/// Mintable Trait for Fungible Token
///
/// The `FungibleMintable` trait extends the `FungibleToken` trait to provide
/// the capability to mint tokens. This trait is designed to be used in
/// conjunction with the `FungibleToken` trait.
///
/// Excluding the `mint` functionality from the
/// [`crate::fungible::FungibleToken`] trait is a deliberate design choice to
/// accommodate flexibility and customization for various smart contract use
/// cases.
pub trait FungibleMintable {
    /// Creates `amount` of tokens and assigns them to `account`. Updates
    /// the total supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new tokens.
    /// * `amount` - The amount of tokens to mint.
    ///
    /// # Errors
    ///
    /// * [`crate::FungibleTokenError::LessThanZero`] - When `amount < 0`.
    /// * [`crate::FungibleTokenError::MathOverflow`] - When `total_supply`
    ///   overflows.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// If you want to add `capped` functionality to this function,
    /// we recommend using [`crate::capped::check_cap()`] when implementing this
    /// function. For more details on the `capped` functionality, check
    /// [`crate::extensions::capped`], and check the `fungible-capped`
    /// example.
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
    /// fn mint(&self, e: &Env, to: Address, amount: i128) {
    ///     // 1. Verify admin has minting privileges (optional)
    ///     let admin = e.storage().instance().get(&ADMIN_KEY).unwrap();
    ///     admin.require_auth();
    ///
    ///     // 2. Only then call the actual mint function
    ///     crate::mintable::mint(e, &to, amount);
    /// }
    /// ```
    ///
    /// Failing to add proper authorization could allow anyone to mint tokens!
    fn mint(e: &Env, to: Address, amount: i128);
}

// ################## EVENTS ##################

/// Emits an event indicating a mint of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
///
/// # Events
///
/// * topics - `["mint", account: Address]`
/// * data - `[amount: i128]`
pub fn emit_mint(e: &Env, to: &Address, amount: i128) {
    let topics = (symbol_short!("mint"), to);
    e.events().publish(topics, amount)
}
