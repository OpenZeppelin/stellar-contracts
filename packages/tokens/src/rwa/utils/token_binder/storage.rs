use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, TryFromVal, Val, Vec};

use crate::rwa::utils::token_binder::{
    TokenBinderError, BUCKET_SIZE, MAX_TOKENS,
    TOKEN_BINDER_EXTEND_AMOUNT, TOKEN_BINDER_TTL_THRESHOLD,
};

#[cfg(not(feature = "certora"))]
use crate::rwa::utils::token_binder::{
    emit_token_bound, emit_token_unbound,
};

/// Storage keys for the token binder system.
///
/// - Tokens are stored in buckets of 100 addresses each
/// - Each bucket is a `Vec<Address>` stored under its bucket index
/// - Total count is tracked separately
/// - When a token is unbound, the last token is moved to fill the gap
///   (swap-remove pattern)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenBinderStorageKey {
    /// Maps bucket index to a vector of token addresses (max 100 per bucket)
    TokenBucket(u32),
    /// Total count of bound tokens
    TotalCount,
}

// ################## QUERY STATE ##################

/// Returns the total number of tokens currently bound to this contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn linked_token_count(e: &Env) -> u32 {
    get_persistent_entry(e, &TokenBinderStorageKey::TotalCount).unwrap_or(0)
}

/// Returns a token address by its global index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `index` - The index of the token to retrieve.
///
/// # Errors
///
/// * [`TokenBinderError::TokenNotFound`] - If `index` is out of bounds.
pub fn get_token_by_index(e: &Env, index: u32) -> Address {
    let count = linked_token_count(e);
    if index >= count {
        panic_with_error!(e, TokenBinderError::TokenNotFound)
    }

    let bucket_index = index / BUCKET_SIZE;
    let offset_in_bucket = index % BUCKET_SIZE;

    let bucket: Vec<Address> =
        get_persistent_entry(e, &TokenBinderStorageKey::TokenBucket(bucket_index))
            .expect("bucket to be present");

    bucket.get(offset_in_bucket).expect("value in bucket to be present")
}

/// Returns the global index of a bound token address.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `token` - The token address to look up.
///
/// # Errors
///
/// * [`TokenBinderError::TokenNotFound`] - If the token is not currently bound.
///
/// # Notes
///
/// Performs a linear scan across all buckets. With Protocol 23, live state
/// reads are inexpensive and read-entry limits have been removed.
pub fn get_token_index(e: &Env, token: &Address) -> u32 {
    let count = linked_token_count(e);
    if count == 0 {
        panic_with_error!(e, TokenBinderError::TokenNotFound)
    }
    let last_bucket = (count - 1) / BUCKET_SIZE;
    for bucket_idx in 0..=last_bucket {
        let bucket: Vec<Address> =
            get_persistent_entry(e, &TokenBinderStorageKey::TokenBucket(bucket_idx))
                .unwrap_or_else(|| Vec::new(e));

        if let Some(relative_index) = bucket.first_index_of(token) {
            return bucket_idx * BUCKET_SIZE + relative_index;
        }
    }
    panic_with_error!(e, TokenBinderError::TokenNotFound)
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
/// Performs a linear scan across all buckets.
pub fn is_token_bound(e: &Env, token: &Address) -> bool {
    let count = linked_token_count(e);
    if count == 0 {
        return false;
    }
    let last_bucket = (count - 1) / BUCKET_SIZE;
    for bucket_idx in 0..=last_bucket {
        let bucket: Vec<Address> =
            get_persistent_entry(e, &TokenBinderStorageKey::TokenBucket(bucket_idx))
                .unwrap_or_else(|| Vec::new(e));
        if bucket.contains(token.clone()) {
            return true;
        }
    }
    false
}

/// Returns all currently bound token addresses in order.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn linked_tokens(e: &Env) -> Vec<Address> {
    let count = linked_token_count(e);
    let mut tokens = Vec::new(e);

    if count == 0 {
        return tokens;
    }

    let last_bucket = (count - 1) / BUCKET_SIZE;
    for bucket_idx in 0..=last_bucket {
        let bucket: Vec<Address> =
            get_persistent_entry(e, &TokenBinderStorageKey::TokenBucket(bucket_idx))
                .unwrap_or_else(|| Vec::new(e));

        tokens.append(&bucket);
    }

    tokens
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
    if is_token_bound(e, token) {
        panic_with_error!(e, TokenBinderError::TokenAlreadyBound)
    }

    let mut count = linked_token_count(e);
    if count >= MAX_TOKENS {
        panic_with_error!(e, TokenBinderError::MaxTokensReached)
    }

    let bucket_index = count / BUCKET_SIZE;
    let key = TokenBinderStorageKey::TokenBucket(bucket_index);
    let mut bucket: Vec<Address> =
        e.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(e));

    bucket.push_back(token.clone());
    e.storage().persistent().set(&key, &bucket);

    count += 1;
    e.storage().persistent().set(&TokenBinderStorageKey::TotalCount, &count);
    #[cfg(not(feature = "certora"))]
    emit_token_bound(e, token);
}

/// Binds multiple token addresses to the contract in a single batch.
///
/// Tokens are appended in order across buckets, spilling into up to 3 buckets
/// as the max. batch size is BUCKET_SIZE * 2.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `tokens` - A vector of token addresses to bind.
///
/// # Errors
///
/// * [`TokenBinderError::BindBatchTooLarge`] - If the batch size exceeds the
///   allowed limit.
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
    let mut count = linked_token_count(e);

    // Enforce batch size and capacity to avoid running out of resources:
    // max. BUCKET_SIZE * 2 tokens allowed in a batch.
    if tokens.len() > BUCKET_SIZE * 2 {
        panic_with_error!(e, TokenBinderError::BindBatchTooLarge)
    }
    if count + tokens.len() > MAX_TOKENS {
        panic_with_error!(e, TokenBinderError::MaxTokensReached)
    }

    // Check for duplicates using Map for O(n) complexity instead of O(n²)
    let mut seen = Map::<Address, ()>::new(e);
    for i in 0..tokens.len() {
        let token = tokens.get_unchecked(i);
        if seen.contains_key(token.clone()) {
            panic_with_error!(e, TokenBinderError::BindBatchDuplicates)
        }
        seen.set(token, ());
    }

    // Build a Map of already-bound tokens for O(1) lookups instead of O(n)
    let already_bound = linked_tokens(e);
    let mut bound_map = Map::<Address, ()>::new(e);
    for i in 0..already_bound.len() {
        bound_map.set(already_bound.get_unchecked(i), ());
    }

    // Fill buckets sequentially until all tokens are stored.
    let mut i: u32 = 0;
    while i < tokens.len() {
        let bucket_index = count / BUCKET_SIZE;
        let key = TokenBinderStorageKey::TokenBucket(bucket_index);
        let mut bucket: Vec<Address> =
            e.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(e));

        // Capacity left in this bucket
        let used = bucket.len();
        let remaining = BUCKET_SIZE - used;
        let to_take = core::cmp::min(remaining, tokens.len() - i);
        let end = i + to_take;

        while i < end {
            let token = tokens.get(i).expect("value to be present");
            if bound_map.contains_key(token.clone()) {
                panic_with_error!(e, TokenBinderError::TokenAlreadyBound)
            }
            bucket.push_back(token.clone());
            #[cfg(not(feature = "certora"))]
            emit_token_bound(e, &token);
            i += 1;
            count += 1;
        }

        // Persist this bucket once per fill
        e.storage().persistent().set(&key, &bucket);
    }

    e.storage().persistent().set(&TokenBinderStorageKey::TotalCount, &count);
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
    let token_index = get_token_index(e, token);

    let count = linked_token_count(e);

    // Can't overflow because `get_token_index()` would panic if count == 0
    let last_index = count - 1;

    if token_index != last_index {
        let last_token = get_token_by_index(e, last_index);

        // Overwrite the removed slot with the last token
        let token_bucket_index = token_index / BUCKET_SIZE;
        let token_offset = token_index % BUCKET_SIZE;
        let token_key = TokenBinderStorageKey::TokenBucket(token_bucket_index);
        let mut token_bucket: Vec<Address> =
            e.storage().persistent().get(&token_key).unwrap_or_else(|| Vec::new(e));
        token_bucket.set(token_offset, last_token.clone());
        e.storage().persistent().set(&token_key, &token_bucket);
    }

    // Remove the last token from its bucket
    let last_bucket_index = last_index / BUCKET_SIZE;
    let last_key = TokenBinderStorageKey::TokenBucket(last_bucket_index);
    let mut last_bucket: Vec<Address> =
        e.storage().persistent().get(&last_key).unwrap_or_else(|| Vec::new(e));
    // if empty pop_back returns None
    last_bucket.pop_back();

    e.storage().persistent().set(&last_key, &last_bucket);

    // Update total count
    e.storage().persistent().set(&TokenBinderStorageKey::TotalCount, &last_index);
    #[cfg(not(feature = "certora"))]
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
