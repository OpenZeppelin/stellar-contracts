use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::utils::token_binder::{
    emit_token_bound, emit_token_unbound, TokenBinderError, TOKEN_BINDER_EXTEND_AMOUNT,
    TOKEN_BINDER_TTL_THRESHOLD,
};

/// Storage keys for the token binder system.
///
/// This implementation follows an enumerable pattern similar to the country profile storage:
/// - Tokens are stored by index (0, 1, 2, ...)
/// - A reverse mapping from token address to index is maintained
/// - Total count is tracked separately
/// - When a token is unbound, the last token is moved to fill the gap (swap-remove pattern)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenBinderStorageKey {
    /// Maps index to token address
    Token(u32),
    /// Maps token address to its index (for O(1) lookup)
    TokenIndex(Address),
    /// Total count of bound tokens
    TotalCount,
}

/// Gets the total number of bound tokens.
pub fn linked_token_count(e: &Env) -> u32 {
    e.storage().persistent().get(&TokenBinderStorageKey::TotalCount).unwrap_or(0)
}

/// Returns a token address by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `index` - The index of the token to retrieve
pub fn get_token_by_index(e: &Env, index: u32) -> Address {
    let key = TokenBinderStorageKey::Token(index);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                TOKEN_BINDER_TTL_THRESHOLD,
                TOKEN_BINDER_EXTEND_AMOUNT,
            )
        })
        .unwrap_or_else(|| panic_with_error!(e, TokenBinderError::TokenNotFound))
}

/// Returns the index of a bound token.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address to look up
pub fn get_token_index(e: &Env, token: &Address) -> u32 {
    let key = TokenBinderStorageKey::TokenIndex(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                TOKEN_BINDER_TTL_THRESHOLD,
                TOKEN_BINDER_EXTEND_AMOUNT,
            )
        })
        .unwrap_or_else(|| panic_with_error!(e, TokenBinderError::TokenNotFound))
}

/// Checks if a token is currently bound.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address to look up
pub fn is_token_bound(e: &Env, token: &Address) -> bool {
    let key = TokenBinderStorageKey::TokenIndex(token.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(
            &key,
            TOKEN_BINDER_TTL_THRESHOLD,
            TOKEN_BINDER_EXTEND_AMOUNT,
        );
        true
    } else {
        false
    }
}

/// Gets all bound tokens as a vector.
///
/// # Arguments
///
/// * `e` - The Soroban environment
pub fn linked_tokens(e: &Env) -> Vec<Address> {
    let count = linked_token_count(e);
    let mut tokens = Vec::new(e);

    for i in 0..count {
        let token = get_token_by_index(e, i);
        tokens.push_back(token);
    }

    tokens
}

/// Binds a token to the contract.
///
/// If the token is already bound, this function does nothing.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address to bind
pub fn bind_token(e: &Env, token: &Address) {
    if is_token_bound(e, token) {
        panic_with_error!(e, TokenBinderError::TokenAlreadyBound)
    }

    let count = linked_token_count(e);

    e.storage().persistent().set(&TokenBinderStorageKey::Token(count), token);

    e.storage().persistent().set(&TokenBinderStorageKey::TokenIndex(token.clone()), &count);

    e.storage().persistent().set(&TokenBinderStorageKey::TotalCount, &(count + 1));

    emit_token_bound(e, token);
}

/// Unbinds a token from the contract.
///
/// Uses a swap-remove pattern: the last token in the list is moved to fill
/// the gap left by the removed token. This keeps the storage compact but
/// means that token indices can change.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address to unbind
pub fn unbind_token(e: &Env, token: &Address) {
    let token_index = get_token_index(e, token);

    let count = linked_token_count(e);

    // Can't overflow because `get_token_index()` would panic if count == 0
    let last_index = count - 1;

    if token_index != last_index {
        let last_token = get_token_by_index(e, last_index);

        e.storage().persistent().set(&TokenBinderStorageKey::Token(token_index), &last_token);
        e.storage().persistent().set(&TokenBinderStorageKey::TokenIndex(last_token), &token_index);
    }

    e.storage().persistent().remove(&TokenBinderStorageKey::TokenIndex(token.clone()));
    e.storage().persistent().remove(&TokenBinderStorageKey::Token(last_index));
    e.storage().persistent().set(&TokenBinderStorageKey::TotalCount, &last_index);

    emit_token_unbound(e, token);
}
