pub mod storage;
mod test;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Mintable Trait for Fungible Token
///
/// The `Mintable` trait extends the `FungibleToken` trait to provide the
/// capability to mint tokens. This trait is designed to be used in conjunction
/// with the `FungibleToken` trait.
///
/// Excluding the `mint` functionality from the `[FungibleToken]` trait
/// is a deliberate design choice to accommodate flexibility and customization
/// for various smart contract use cases.
#[contractclient(name = "FungibleMintableClient")]
pub trait FungibleMintable {
    /// Creates `amount` of tokens and assigns them to `account`. Updates
    /// the total supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address receiving the new tokens.
    /// * `amount` - The amount of tokens to mint.
    ///
    /// # Events
    ///
    /// * topics - `["mint", account: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::extensions::mintable::storage::mint()`]
    /// function from the `storage` module when implementing this function.
    ///
    /// IMPORTANT: Please do not forget that, you probably will want to have
    /// some authorization controls for minting tokens.
    fn mint(e: &Env, account: &Address, amount: i128);
}
// ################## EVENTS ##################

/// Emits an event indicating a mint of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
///
/// # Events
///
/// * topics - `["mint", account: Address]`
/// * data - `[amount: i128]`
pub fn emit_mint(e: &Env, account: &Address, amount: i128) {
    let topics = (symbol_short!("mint"), account);
    e.events().publish(topics, amount)
}
