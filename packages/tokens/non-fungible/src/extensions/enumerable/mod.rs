/// This module provides enumeration capabilities for NFT contracts.
/// 
/// The enumeration extension allows contracts to:
/// - Track and query the total supply of tokens
/// - List all tokens owned by a specific address
/// - List all tokens in the contract
/// 
/// This extension is particularly useful for:
/// - Marketplaces that need to display all NFTs in a collection
/// - Wallets that need to show all NFTs owned by an address
/// - Applications that need to iterate over tokens
///
/// # Implementation Notes
///
/// The extension maintains three types of storage:
/// - Total supply: Tracks the total number of tokens in circulation
/// - Per-owner token lists: Maps addresses to their owned token IDs
/// - Global token list: Contains all token IDs in the contract
///
/// All storage is automatically managed when using the provided functions
/// for minting, transferring, and burning tokens.
///
/// # Example
/// ```rust
/// use soroban_sdk::{Address, Env};
/// use crate::extensions::enumerable::{NonFungibleEnumerable, storage::add_token};
///
/// // Implement the trait for your contract
/// impl NonFungibleEnumerable for MyNFTContract {
///     fn total_supply(e: &Env) -> u128 {
///         enumerable::storage::total_supply(e)
///     }
///     // ... other implementations ...
/// }
///
/// // Use in your contract functions
/// fn mint(e: &Env, to: Address, token_id: u128) {
///     // ... your minting logic ...
///     add_token(e, token_id, &to);
/// }
/// ```
use soroban_sdk::{Address, Env, Vec};

mod storage;
mod test;

pub use storage::{add_token, update_token_lists};
use crate::storage::owner_of;

/// Extension trait that provides enumeration capabilities for NFTs.
///
/// This trait should be implemented by contracts that want to support
/// token enumeration. It provides methods to query the total supply and
/// list tokens either globally or per owner.
pub trait NonFungibleEnumerable {
    /// Returns the total number of tokens stored by the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment handle
    fn total_supply(e: &Env) -> u128;

    /// Returns a list of token IDs owned by `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment handle
    /// * `owner` - The address whose tokens to list
    fn tokens_of_owner(e: &Env, owner: Address) -> Vec<u128>;

    /// Returns a list of all token IDs in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment handle
    fn all_tokens(e: &Env) -> Vec<u128>;
}

/// Helper functions for managing token enumeration.
///
/// These functions should be used by contracts implementing the
/// NonFungibleEnumerable trait to maintain token lists and ownership
/// information.
pub mod helpers {
    use super::*;

    /// Adds a token to the enumeration tracking when it is minted.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment handle
    /// * `token_id` - The ID of the token being minted
    /// * `to` - The address that will own the token
    pub fn on_mint(e: &Env, token_id: u128, to: &Address) {
        add_token(e, token_id, to);
    }

    /// Updates token enumeration tracking when a token is transferred.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment handle
    /// * `token_id` - The ID of the token being transferred
    /// * `from` - The current owner's address
    /// * `to` - The recipient's address
    pub fn on_transfer(e: &Env, token_id: u128, from: &Address, to: &Address) {
        update_token_lists(e, token_id, Some(from), Some(to));
    }

    /// Updates token enumeration tracking when a token is burned.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment handle
    /// * `token_id` - The ID of the token being burned
    pub fn on_burn(e: &Env, token_id: u128) {
        let owner = owner_of(e, token_id);
        update_token_lists(e, token_id, Some(&owner), None);
    }
} 