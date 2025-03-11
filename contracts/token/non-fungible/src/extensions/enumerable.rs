use soroban_sdk::{contracttype, Address, Env, Vec};

/// Storage keys for enumerable data
#[contracttype]
#[derive(Clone)]
pub enum EnumerableDataKey {
    AllTokens,              // Vector of all token IDs
    TokensIndex(u128),      // Index of token in AllTokens array
    OwnerTokens(Address),   // List of tokens owned by address
    OwnerTokensIndex(Address, u128),  // Index of token in owner's array
}

/// Core enumerable interface
pub trait NonFungibleEnumerable {
    /// Returns the total amount of tokens stored by the contract
    fn total_supply(e: &Env) -> u128;

    /// Returns a token ID at a given index of all the tokens stored by the contract
    fn token_by_index(e: &Env, index: u128) -> u128;

    /// Returns a token ID owned by `owner` at a given index of its token list
    fn token_of_owner_by_index(e: &Env, owner: Address, index: u128) -> u128;
}

/// Helper functions for managing enumerable storage
pub struct EnumerableStorage;

impl EnumerableStorage {
    /// Add a token to the global list of tokens
    pub fn add_token(e: &Env, token_id: u128) {
        let mut all_tokens = Self::get_all_tokens(e);
        let new_index = all_tokens.len();
        all_tokens.push_back(token_id);
        
        e.storage().instance().set(&EnumerableDataKey::AllTokens, &all_tokens);
        e.storage().instance().set(&EnumerableDataKey::TokensIndex(token_id), &new_index);
    }

    /// Add a token to an owner's list of tokens
    pub fn add_token_to_owner(e: &Env, owner: &Address, token_id: u128) {
        let mut owner_tokens = Self::get_owner_tokens(e, owner);
        let new_index = owner_tokens.len();
        owner_tokens.push_back(token_id);
        
        e.storage().instance().set(&EnumerableDataKey::OwnerTokens(owner.clone()), &owner_tokens);
        e.storage().instance().set(&EnumerableDataKey::OwnerTokensIndex(owner.clone(), token_id), &new_index);
    }

    /// Remove a token from the global list of tokens
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

    /// Remove a token from an owner's list of tokens
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

    /// Get all tokens
    pub fn get_all_tokens(e: &Env) -> Vec<u128> {
        e.storage().instance().get(&EnumerableDataKey::AllTokens)
            .unwrap_or(Vec::new(e))
    }

    /// Get all tokens owned by an address
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
} 