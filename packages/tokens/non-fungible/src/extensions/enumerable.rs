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
/// use crate::extensions::enumerable::{NonFungibleEnumerable, add_token};
///
/// // Implement the trait for your contract
/// impl NonFungibleEnumerable for MyNFTContract {
///     fn total_supply(e: &Env) -> u128 {
///         enumerable::total_supply(e)
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
use soroban_sdk::{contracttype, Address, Env, Vec};
use crate::storage::{StorageKey, owner_of};

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

/// Storage key for enumerable extension data.
///
/// This enum defines the storage keys used by the enumerable extension
/// to maintain token lists and ownership information.
#[contracttype]
pub enum EnumerableStorageKey {
    /// Key for storing token IDs owned by an address
    TokensOfOwner(Address),
    /// Key for storing all token IDs
    AllTokens,
}

/// Returns the total number of tokens stored by the contract.
///
/// # Arguments
///
/// * `e` - The environment handle
///
/// # Returns
///
/// The total number of tokens in circulation. Returns 0 if no tokens exist.
pub fn total_supply(e: &Env) -> u128 {
    e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0)
}

/// Returns a list of token IDs owned by `owner`.
///
/// # Arguments
///
/// * `e` - The environment handle
/// * `owner` - The address whose tokens to list
///
/// # Returns
///
/// A vector containing the token IDs owned by the address.
/// Returns an empty vector if the address owns no tokens.
pub fn tokens_of_owner(e: &Env, owner: &Address) -> Vec<u128> {
    e.storage().persistent().get(&EnumerableStorageKey::TokensOfOwner(owner.clone())).unwrap_or(Vec::new(e))
}

/// Returns a list of all token IDs in the contract.
///
/// # Arguments
///
/// * `e` - The environment handle
///
/// # Returns
///
/// A vector containing all token IDs in the contract.
/// Returns an empty vector if no tokens exist.
pub fn all_tokens(e: &Env) -> Vec<u128> {
    e.storage().persistent().get(&EnumerableStorageKey::AllTokens).unwrap_or(Vec::new(e))
}

/// Updates the token lists when a token is minted.
///
/// This function should be called after minting a new token to:
/// - Increment the total supply
/// - Add the token to the owner's token list
/// - Add the token to the global token list
///
/// # Arguments
///
/// * `e` - The environment handle
/// * `token_id` - The ID of the token being minted
/// * `to` - The address that will own the token
pub fn add_token(e: &Env, token_id: u128, to: &Address) {
    // Update total supply
    let new_supply = total_supply(e) + 1;
    e.storage().instance().set(&StorageKey::TotalSupply, &new_supply);

    // Update owner's token list
    let mut owner_tokens = tokens_of_owner(e, to);
    owner_tokens.push_back(token_id);
    e.storage().persistent().set(&EnumerableStorageKey::TokensOfOwner(to.clone()), &owner_tokens);

    // Update all tokens list
    let mut all = all_tokens(e);
    all.push_back(token_id);
    e.storage().persistent().set(&EnumerableStorageKey::AllTokens, &all);
}

/// Updates the token lists when a token is transferred or burned.
///
/// This function should be called during token transfers to:
/// - Remove the token from the previous owner's list (if applicable)
/// - Add the token to the new owner's list (for transfers)
/// - Update the total supply and global list (for burns)
///
/// # Arguments
///
/// * `e` - The environment handle
/// * `token_id` - The ID of the token being transferred
/// * `from` - The current owner's address (None if minting)
/// * `to` - The recipient's address (None if burning)
pub fn update_token_lists(e: &Env, token_id: u128, from: Option<&Address>, to: Option<&Address>) {
    if let Some(from_addr) = from {
        // Remove from previous owner's list
        let mut from_tokens = tokens_of_owner(e, from_addr);
        let pos = from_tokens.binary_search(&token_id).unwrap_or_else(|x| x);
        from_tokens.remove(pos);
        e.storage().persistent().set(&EnumerableStorageKey::TokensOfOwner(from_addr.clone()), &from_tokens);
    }

    if let Some(to_addr) = to {
        // Add to new owner's list
        let mut to_tokens = tokens_of_owner(e, to_addr);
        to_tokens.push_back(token_id);
        e.storage().persistent().set(&EnumerableStorageKey::TokensOfOwner(to_addr.clone()), &to_tokens);
    } else {
        // Token is being burned, update total supply and all tokens list
        let new_supply = total_supply(e) - 1;
        e.storage().instance().set(&StorageKey::TotalSupply, &new_supply);

        let mut all = all_tokens(e);
        let pos = all.binary_search(&token_id).unwrap_or_else(|x| x);
        all.remove(pos);
        e.storage().persistent().set(&EnumerableStorageKey::AllTokens, &all);
    }
} 