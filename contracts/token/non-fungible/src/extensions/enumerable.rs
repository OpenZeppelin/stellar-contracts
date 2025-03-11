//! # Enumerable Extension for Non-Fungible Tokens
//!
//! This extension enables NFT contracts to expose their full list of tokens and make them
//! discoverable. It provides efficient storage and retrieval mechanisms for token enumeration.
//!
//! ## Storage Structure
//!
//! The extension uses the following storage keys:
//! - `AllTokens`: Vector storing all token IDs
//! - `TokensIndex`: Maps token ID to its index in the AllTokens array
//! - `OwnerTokens`: Maps owner address to their token IDs
//! - `OwnerTokensIndex`: Maps (owner, token_id) to token's index in owner's array
//!
//! ## Performance Considerations
//! - All operations are O(1) except for initial retrieval of lists
//! - Token removal maintains array density by moving last element to removed position
//! - Uses efficient index mapping for quick lookups
//!
//! ## Error Handling
//! - Returns NonExistentToken error for invalid indices
//! - Panics on removal of non-existent tokens
//! - Returns empty vector for owners with no tokens

use soroban_sdk::{contracttype, Address, Env, Vec};

/// Storage keys for the enumerable extension's data structures
#[contracttype]
#[derive(Clone)]
pub enum EnumerableDataKey {
    /// Vector containing all token IDs in the contract
    AllTokens,
    /// Maps a token ID to its index in the AllTokens array
    TokensIndex(u128),
    /// Maps an owner's address to their list of owned token IDs
    OwnerTokens(Address),
    /// Maps (owner, token_id) pair to the token's index in the owner's token array
    OwnerTokensIndex(Address, u128),
}

/// Core interface for NFT enumeration functionality
///
/// This trait provides methods to enumerate and discover tokens within an NFT contract.
/// Implementations should ensure efficient access to token lists and maintain proper indexing.
pub trait NonFungibleEnumerable {
    /// Returns the total number of tokens stored by the contract
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    ///
    /// # Returns
    ///
    /// The total number of tokens as u128
    fn total_supply(e: &Env) -> u128;

    /// Returns a token ID at a given index of all the tokens stored by the contract
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `index` - The index in the global token list
    ///
    /// # Returns
    ///
    /// The token ID at the specified index
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds
    fn token_by_index(e: &Env, index: u128) -> u128;

    /// Returns a token ID owned by `owner` at a given index of its token list
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `owner` - The address of the token owner
    /// * `index` - The index in the owner's token list
    ///
    /// # Returns
    ///
    /// The token ID at the specified index in the owner's list
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds
    fn token_of_owner_by_index(e: &Env, owner: Address, index: u128) -> u128;
}

/// Helper struct providing storage management functions for enumerable tokens
pub struct EnumerableStorage;

impl EnumerableStorage {
    /// Adds a token to the global list of tokens
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `token_id` - The ID of the token to add
    ///
    /// # Example
    ///
    /// ```rust
    /// EnumerableStorage::add_token(&env, 1);
    /// ```
    pub fn add_token(e: &Env, token_id: u128) {
        let mut all_tokens = Self::get_all_tokens(e);
        let new_index = all_tokens.len();
        all_tokens.push_back(token_id);
        
        e.storage().instance().set(&EnumerableDataKey::AllTokens, &all_tokens);
        e.storage().instance().set(&EnumerableDataKey::TokensIndex(token_id), &new_index);
    }

    /// Adds a token to an owner's list of tokens
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `owner` - The address of the token owner
    /// * `token_id` - The ID of the token to add
    ///
    /// # Example
    ///
    /// ```rust
    /// EnumerableStorage::add_token_to_owner(&env, &owner_address, 1);
    /// ```
    pub fn add_token_to_owner(e: &Env, owner: &Address, token_id: u128) {
        let mut owner_tokens = Self::get_owner_tokens(e, owner);
        let new_index = owner_tokens.len();
        owner_tokens.push_back(token_id);
        
        e.storage().instance().set(&EnumerableDataKey::OwnerTokens(owner.clone()), &owner_tokens);
        e.storage().instance().set(&EnumerableDataKey::OwnerTokensIndex(owner.clone(), token_id), &new_index);
    }

    /// Removes a token from the global list of tokens
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `token_id` - The ID of the token to remove
    ///
    /// # Panics
    ///
    /// Panics if the token is not found in the index
    ///
    /// # Example
    ///
    /// ```rust
    /// EnumerableStorage::remove_token(&env, 1);
    /// ```
    pub fn remove_token(e: &Env, token_id: u128) {
        let mut all_tokens = Self::get_all_tokens(e);
        let index = e.storage().instance().get::<_, u128>(&EnumerableDataKey::TokensIndex(token_id))
            .expect("Token not found in index");
        
        // Move the last token to the deleted token's position
        if index != all_tokens.len() - 1 {
            let last_token_id = all_tokens.get_unchecked(all_tokens.len() - 1);
            all_tokens.set(index, last_token_id);
            e.storage().instance().set(&EnumerableDataKey::TokensIndex(last_token_id), &index);
        }
        
        all_tokens.pop_back();
        e.storage().instance().remove(&EnumerableDataKey::TokensIndex(token_id));
        e.storage().instance().set(&EnumerableDataKey::AllTokens, &all_tokens);
    }

    /// Removes a token from an owner's list of tokens
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `owner` - The address of the token owner
    /// * `token_id` - The ID of the token to remove
    ///
    /// # Panics
    ///
    /// Panics if the token is not found in the owner's index
    ///
    /// # Example
    ///
    /// ```rust
    /// EnumerableStorage::remove_token_from_owner(&env, &owner_address, 1);
    /// ```
    pub fn remove_token_from_owner(e: &Env, owner: &Address, token_id: u128) {
        let mut owner_tokens = Self::get_owner_tokens(e, owner);
        let index = e.storage().instance()
            .get::<_, u128>(&EnumerableDataKey::OwnerTokensIndex(owner.clone(), token_id))
            .expect("Token not found in owner's index");
        
        // Move the last token to the deleted token's position
        if index != owner_tokens.len() - 1 {
            let last_token_id = owner_tokens.get_unchecked(owner_tokens.len() - 1);
            owner_tokens.set(index, last_token_id);
            e.storage().instance().set(
                &EnumerableDataKey::OwnerTokensIndex(owner.clone(), last_token_id),
                &index,
            );
        }
        
        owner_tokens.pop_back();
        e.storage().instance().remove(&EnumerableDataKey::OwnerTokensIndex(owner.clone(), token_id));
        e.storage().instance().set(&EnumerableDataKey::OwnerTokens(owner.clone()), &owner_tokens);
    }

    /// Retrieves all tokens in the contract
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    ///
    /// # Returns
    ///
    /// A vector containing all token IDs, or an empty vector if no tokens exist
    pub fn get_all_tokens(e: &Env) -> Vec<u128> {
        e.storage().instance().get(&EnumerableDataKey::AllTokens)
            .unwrap_or(Vec::new(e))
    }

    /// Retrieves all tokens owned by an address
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference
    /// * `owner` - The address of the token owner
    ///
    /// # Returns
    ///
    /// A vector containing all token IDs owned by the address, or an empty vector if none exist
    pub fn get_owner_tokens(e: &Env, owner: &Address) -> Vec<u128> {
        e.storage().instance().get(&EnumerableDataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(e))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_enumerable_storage() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let token_id = 1u128;

        // Test adding token
        EnumerableStorage::add_token(&env, token_id);
        EnumerableStorage::add_token_to_owner(&env, &owner, token_id);

        // Verify token lists
        let all_tokens = EnumerableStorage::get_all_tokens(&env);
        let owner_tokens = EnumerableStorage::get_owner_tokens(&env, &owner);
        assert_eq!(all_tokens.len(), 1);
        assert_eq!(owner_tokens.len(), 1);
        assert_eq!(all_tokens.get_unchecked(0), token_id);
        assert_eq!(owner_tokens.get_unchecked(0), token_id);

        // Test removing token
        EnumerableStorage::remove_token(&env, token_id);
        EnumerableStorage::remove_token_from_owner(&env, &owner, token_id);

        // Verify token lists are empty
        let all_tokens = EnumerableStorage::get_all_tokens(&env);
        let owner_tokens = EnumerableStorage::get_owner_tokens(&env, &owner);
        assert_eq!(all_tokens.len(), 0);
        assert_eq!(owner_tokens.len(), 0);
    }

    #[test]
    fn test_multiple_tokens() {
        let env = Env::default();
        let owner = Address::generate(&env);
        
        // Add multiple tokens
        for i in 1..=3 {
            EnumerableStorage::add_token(&env, i);
            EnumerableStorage::add_token_to_owner(&env, &owner, i);
        }
        
        let all_tokens = EnumerableStorage::get_all_tokens(&env);
        assert_eq!(all_tokens.len(), 3);
        assert_eq!(all_tokens.get_unchecked(0), 1);
        assert_eq!(all_tokens.get_unchecked(1), 2);
        assert_eq!(all_tokens.get_unchecked(2), 3);
    }

    #[test]
    fn test_multiple_owners() {
        let env = Env::default();
        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        
        EnumerableStorage::add_token(&env, 1);
        EnumerableStorage::add_token(&env, 2);
        
        EnumerableStorage::add_token_to_owner(&env, &owner1, 1);
        EnumerableStorage::add_token_to_owner(&env, &owner2, 2);
        
        let owner1_tokens = EnumerableStorage::get_owner_tokens(&env, &owner1);
        let owner2_tokens = EnumerableStorage::get_owner_tokens(&env, &owner2);
        
        assert_eq!(owner1_tokens.len(), 1);
        assert_eq!(owner2_tokens.len(), 1);
        assert_eq!(owner1_tokens.get_unchecked(0), 1);
        assert_eq!(owner2_tokens.get_unchecked(0), 2);
    }

    #[test]
    #[should_panic(expected = "Token not found in index")]
    fn test_remove_nonexistent_token() {
        let env = Env::default();
        EnumerableStorage::remove_token(&env, 999);
    }
}
