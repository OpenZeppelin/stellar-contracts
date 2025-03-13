use soroban_sdk::{contracttype, Address, Env, Vec};
use crate::storage::{StorageKey, owner_of};

/// Extension trait that provides enumeration capabilities for NFTs
pub trait NonFungibleEnumerable {
    /// Returns the total number of tokens stored by the contract.
    fn total_supply(e: &Env) -> u128;

    /// Returns a list of token IDs owned by `owner`.
    fn tokens_of_owner(e: &Env, owner: Address) -> Vec<u128>;

    /// Returns a list of all token IDs in the contract.
    fn all_tokens(e: &Env) -> Vec<u128>;
}

/// Storage key for enumerable extension data
#[contracttype]
pub enum EnumerableStorageKey {
    /// Key for storing token IDs owned by an address
    TokensOfOwner(Address),
    /// Key for storing all token IDs
    AllTokens,
}

/// Returns the total number of tokens stored by the contract.
pub fn total_supply(e: &Env) -> u128 {
    e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0)
}

/// Returns a list of token IDs owned by `owner`.
pub fn tokens_of_owner(e: &Env, owner: &Address) -> Vec<u128> {
    e.storage().persistent().get(&EnumerableStorageKey::TokensOfOwner(owner.clone())).unwrap_or(Vec::new(e))
}

/// Returns a list of all token IDs in the contract.
pub fn all_tokens(e: &Env) -> Vec<u128> {
    e.storage().persistent().get(&EnumerableStorageKey::AllTokens).unwrap_or(Vec::new(e))
}

/// Updates the token lists when a token is minted
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

/// Updates the token lists when a token is transferred
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