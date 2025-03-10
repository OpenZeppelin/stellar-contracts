#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

/// Error codes for NFT operations
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    TokenNotFound = 1,
    TokenAlreadyExists = 2,
    InvalidTokenId = 3,
}

/// Storage keys for enumerable data
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    AllTokens,              // Vector of all token IDs
    TokenOwner(u32),       // Maps token_id to owner
    OwnerTokens(Address),  // Maps owner to their tokens
    TokenCount,            // Total number of tokens
}

/// Core enumerable interface
pub trait NonFungibleEnumerable {
    /// Returns all existing token IDs
    fn all_tokens(env: Env) -> Vec<u32>;
    
    /// Returns all tokens owned by an address
    fn tokens_of_owner(env: Env, owner: Address) -> Vec<u32>;
    
    /// Returns the owner of a specific token
    fn owner_of(env: Env, token_id: u32) -> Result<Address, Error>;
    
    /// Returns the total number of tokens
    fn total_supply(env: Env) -> u32;
}

#[contract]
pub struct NonFungibleToken;

#[contractimpl]
impl NonFungibleEnumerable for NonFungibleToken {
    fn all_tokens(env: Env) -> Vec<u32> {
        env.storage()
            .instance()
            .get(&DataKey::AllTokens)
            .unwrap_or(Vec::new(&env))
    }

    fn tokens_of_owner(env: Env, owner: Address) -> Vec<u32> {
        env.storage()
            .instance()
            .get(&DataKey::OwnerTokens(owner))
            .unwrap_or(Vec::new(&env))
    }

    fn owner_of(env: Env, token_id: u32) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::TokenOwner(token_id))
            .ok_or(Error::TokenNotFound)
    }

    fn total_supply(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::TokenCount)
            .unwrap_or(0)
    }
}

// Internal helper functions
#[contractimpl]
impl NonFungibleToken {
    /// Initialize the contract
    pub fn initialize(env: Env) {
        env.storage().instance().set(&DataKey::TokenCount, &0u32);
    }

    /// Add a new token to the enumerable collection
    pub fn add_token(env: Env, owner: Address, token_id: u32) -> Result<(), Error> {
        // Check if token already exists
        if env.storage().instance().has(&DataKey::TokenOwner(token_id)) {
            return Err(Error::TokenAlreadyExists);
        }

        // Add token to global list
        let mut all_tokens = Self::all_tokens(env.clone());
        all_tokens.push_back(token_id);
        env.storage().instance().set(&DataKey::AllTokens, &all_tokens);

        // Add token to owner's collection
        let mut owner_tokens = Self::tokens_of_owner(env.clone(), owner.clone());
        owner_tokens.push_back(token_id);
        env.storage().instance().set(&DataKey::OwnerTokens(owner.clone()), &owner_tokens);
        env.storage().instance().set(&DataKey::TokenOwner(token_id), &owner);

        // Update total count
        let current_count = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TokenCount, &(current_count + 1));

        Ok(())
    }

    /// Remove a token from the enumerable collection
    pub fn remove_token(env: Env, token_id: u32) -> Result<(), Error> {
        // Get token owner
        let owner = Self::owner_of(env.clone(), token_id)?;

        // Remove from global list
        let mut all_tokens = Self::all_tokens(env.clone());
        all_tokens.remove(token_id);
        env.storage().instance().set(&DataKey::AllTokens, &all_tokens);

        // Remove from owner's collection
        let mut owner_tokens = Self::tokens_of_owner(env.clone(), owner.clone());
        owner_tokens.remove(token_id);
        env.storage().instance().set(&DataKey::OwnerTokens(owner), &owner_tokens);
        
        // Remove ownership record
        env.storage().instance().remove(&DataKey::TokenOwner(token_id));

        // Update total count
        let current_count = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TokenCount, &(current_count - 1));

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_enumerable() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NonFungibleToken);
        let client = NonFungibleTokenClient::new(&env, &contract_id);

        // Initialize contract
        client.initialize();

        // Create test address
        let owner = Address::generate(&env);

        // Add tokens
        client.add_token(&owner, &1).unwrap();
        client.add_token(&owner, &2).unwrap();

        // Test enumerable functions
        assert_eq!(client.total_supply(), 2);
        assert_eq!(client.all_tokens(), vec![&env, 1, 2]);
        assert_eq!(client.tokens_of_owner(&owner), vec![&env, 1, 2]);
        assert_eq!(client.owner_of(&1).unwrap(), owner);

        // Test token removal
        client.remove_token(&1).unwrap();
        assert_eq!(client.total_supply(), 1);
        assert_eq!(client.all_tokens(), vec![&env, 2]);
        assert_eq!(client.tokens_of_owner(&owner), vec![&env, 2]);
        assert!(client.owner_of(&1).is_err());
    }
} 