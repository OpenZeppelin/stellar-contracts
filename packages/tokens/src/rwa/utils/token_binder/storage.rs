use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, TryFromVal, Val, Vec};

use crate::rwa::utils::token_binder::{
    emit_token_bound, emit_token_unbound, TokenBinderError, MAX_TOKENS, TOKEN_BINDER_EXTEND_AMOUNT,
    TOKEN_BINDER_TTL_THRESHOLD,
};

/// Storage keys for the token binder system.
///
/// All bound token addresses are kept in a single `Vec<Address>` entry. With
/// the capacity capped at [`MAX_TOKENS`], the full list stays a few kilobytes,
/// far below the ledger's per-entry size limit. When a token is unbound, the
/// last token is moved to fill the gap (swap-remove pattern).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenBinderStorageKey {
    /// The list of all bound token addresses.
    Tokens,
}

// ################## QUERY STATE ##################

/// Returns all currently bound token addresses in order.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn linked_tokens(e: &Env) -> Vec<Address> {
    get_persistent_entry(e, &TokenBinderStorageKey::Tokens).unwrap_or_else(|| Vec::new(e))
}

/// Returns the total number of tokens currently bound to this contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn linked_token_count(e: &Env) -> u32 {
    linked_tokens(e).len()
}

/// Checks whether a token address is currently bound.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `token` - The token address to look up.
///
/// # Notes
///
/// Performs a linear scan of the token list.
pub fn is_token_bound(e: &Env, token: &Address) -> bool {
    linked_tokens(e).contains(token.clone())
}

// ################## CHANGE STATE ##################

/// Binds a single token address to the contract.
///
/// If the token is already bound, this function panics.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `token` - The token address to bind.
///
/// # Errors
///
/// * [`TokenBinderError::TokenAlreadyBound`] - If the token is already bound.
/// * [`TokenBinderError::MaxTokensReached`] - If capacity has been reached.
///
/// # Events
///
/// * topics - `["token_bound", token: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn bind_token(e: &Env, token: &Address) {
    let mut tokens = linked_tokens(e);
    if tokens.contains(token.clone()) {
        panic_with_error!(e, TokenBinderError::TokenAlreadyBound)
    }
    if tokens.len() >= MAX_TOKENS {
        panic_with_error!(e, TokenBinderError::MaxTokensReached)
    }

    tokens.push_back(token.clone());
    e.storage().persistent().set(&TokenBinderStorageKey::Tokens, &tokens);

    emit_token_bound(e, token);
}

/// Binds multiple token addresses to the contract in a single batch.
///
/// The batch is appended in order; capacity is bounded by [`MAX_TOKENS`], so
/// a single call can bind up to the full remaining capacity.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `tokens` - A vector of token addresses to bind.
///
/// # Errors
///
/// * [`TokenBinderError::MaxTokensReached`] - If capacity is exceeded.
/// * [`TokenBinderError::BindBatchDuplicates`] - If the batch contains
///   duplicate addresses.
/// * [`TokenBinderError::TokenAlreadyBound`] - If any token in the batch is
///   already bound.
///
/// # Events
///
/// Emits per-token events as each token is bound:
/// * topics - `["token_bound", token: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn bind_tokens(e: &Env, tokens: &Vec<Address>) {
    let mut bound = linked_tokens(e);
    if bound.len() + tokens.len() > MAX_TOKENS {
        panic_with_error!(e, TokenBinderError::MaxTokensReached)
    }

    // Check for duplicates using Map for O(n) complexity instead of O(n²)
    let mut seen = Map::<Address, ()>::new(e);
    for token in tokens.iter() {
        if seen.contains_key(token.clone()) {
            panic_with_error!(e, TokenBinderError::BindBatchDuplicates)
        }
        seen.set(token, ());
    }

    // Build a Map of already-bound tokens for O(1) lookups instead of O(n)
    let mut bound_map = Map::<Address, ()>::new(e);
    for token in bound.iter() {
        bound_map.set(token, ());
    }

    for token in tokens.iter() {
        if bound_map.contains_key(token.clone()) {
            panic_with_error!(e, TokenBinderError::TokenAlreadyBound)
        }
        bound.push_back(token.clone());
        emit_token_bound(e, &token);
    }

    e.storage().persistent().set(&TokenBinderStorageKey::Tokens, &bound);
}

/// Unbinds a single token address from the contract.
///
/// Uses a swap-remove pattern: the last token in the list is moved to fill
/// the gap left by the removed token. This keeps the storage compact but
/// means that token indices can change.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `token` - The token address to unbind.
///
/// # Errors
///
/// * [`TokenBinderError::TokenNotFound`] - If the token is not currently bound.
///
/// # Events
///
/// * topics - `["token_unbound", token: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn unbind_token(e: &Env, token: &Address) {
    let mut tokens = linked_tokens(e);
    let index = tokens
        .first_index_of(token)
        .unwrap_or_else(|| panic_with_error!(e, TokenBinderError::TokenNotFound));

    // Can't overflow because `first_index_of` would have returned `None` on
    // an empty list
    let last_index = tokens.len() - 1;

    if index != last_index {
        // Overwrite the removed slot with the last token
        let last_token = tokens.get_unchecked(last_index);
        tokens.set(index, last_token);
    }
    tokens.pop_back();

    e.storage().persistent().set(&TokenBinderStorageKey::Tokens, &tokens);

    emit_token_unbound(e, token);
}

// ################## HELPERS ##################

/// Helper function that tries to retrieve a persistent storage value and
/// extend its TTL if the entry exists.
///
/// # Arguments
///
/// * `e` - The Soroban reference.
/// * `key` - The key required to retrieve the underlying storage.
fn get_persistent_entry<T: TryFromVal<Env, Val>>(
    e: &Env,
    key: &TokenBinderStorageKey,
) -> Option<T> {
    e.storage().persistent().get::<_, T>(key).inspect(|_| {
        e.storage().persistent().extend_ttl(
            key,
            TOKEN_BINDER_TTL_THRESHOLD,
            TOKEN_BINDER_EXTEND_AMOUNT,
        );
    })
}
