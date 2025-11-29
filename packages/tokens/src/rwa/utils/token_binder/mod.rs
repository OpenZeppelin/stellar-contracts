mod storage;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{contractclient, contracterror, Address, Env, Vec};
pub use storage::{
    bind_token, bind_tokens, get_token_by_index, get_token_index, is_token_bound, linked_tokens,
    unbind_token,
};

/// Trait for managing token bindings to periphery contracts.
///
/// The `TokenBinder` trait provides a standardized interface for linking tokens
/// to periphery contracts requiring this, such as:
/// - Identity Storage Registry
/// - Compliance contracts
///
/// This binding mechanism allows tokens to be associated with regulatory and
/// compliance infrastructure, enabling features like identity verification,
/// compliance checking, and claim validation.
///
/// # Storage Pattern
///
/// The underlying storage uses an enumerable pattern for efficiency:
/// - Tokens are indexed sequentially (0, 1, 2, ...)
/// - Swap-remove pattern maintains compact storage when unbinding
///
/// Note that the storage module also exposes a batch binding helper
/// `bind_tokens(e, tokens)` which is not part of this trait, so that client
/// contracts can decide how to expose batch semantics in their own interfaces.
///
/// Implementation notes:
/// - Token addresses are stored in buckets of 100 addresses each (`BUCKET_SIZE
///   = 100`).
/// - Up to 100 buckets are supported (`MAX_BUCKETS = 100`), allowing at most
///   10,000 tokens bound to a single contract.
/// - With Protocol 23, reading live Soroban state is inexpensive and read-entry
///   limits per transaction have been removed. Lookups are therefore cheap, and
///   we keep storage simple with no reverse mapping; functions like
///   `get_token_index()` linearly scan buckets.
#[contractclient(name = "TokenBinderClient")]
pub trait TokenBinder {
    /// Returns all currently bound token addresses.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment
    fn linked_tokens(e: &Env) -> Vec<Address>;

    /// Binds a token to this contract's periphery services.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment
    /// * `token` - The token address to bind
    /// * `operator` - The address authorizing this operation
    ///
    /// # Security Note
    ///
    /// Implementations should include proper authorization checks to ensure
    /// only authorized operators can bind tokens.
    fn bind_token(e: &Env, token: Address, operator: Address);

    /// Unbinds a token from this contract's periphery services.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment
    /// * `token` - The token address to unbind
    /// * `operator` - The address authorizing this operation
    ///
    /// # Security Note
    ///
    /// Implementations should include proper authorization checks to ensure
    /// only authorized operators can unbind tokens.
    fn unbind_token(e: &Env, token: Address, operator: Address);
}

// ################## ERRORS ##################

/// Error codes for the Token Binder system.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TokenBinderError {
    /// The specified token was not found in the bound tokens list.
    TokenNotFound = 330,
    /// Attempted to bind a token that is already bound.
    TokenAlreadyBound = 331,
    /// Total token capacity (MAX_TOKENS) has been reached.
    MaxTokensReached = 332,
    /// Batch bind size exceeded.
    BindBatchTooLarge = 333,
    /// The batch contains duplicates.
    BindBatchDuplicates = 334,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const TOKEN_BINDER_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const TOKEN_BINDER_TTL_THRESHOLD: u32 = TOKEN_BINDER_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Number of Token addresses in bucket
pub const BUCKET_SIZE: u32 = 100;
/// Max. number of buckets
pub const MAX_BUCKETS: u32 = 100;
/// Max. number of Token addresses
pub const MAX_TOKENS: u32 = BUCKET_SIZE * MAX_BUCKETS; // 10_000

// ################## EVENTS ##################

/// Event emitted when a token is bound to the contract.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenBound {
    #[topic]
    pub token: Address,
}

/// Emits an event when a token is bound to the contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address that was bound
#[cfg(not(feature = "certora"))]
pub fn emit_token_bound(e: &Env, token: &Address) {
    TokenBound { token: token.clone() }.publish(e);
}

/// Event emitted when a token is unbound from the contract.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenUnbound {
    #[topic]
    pub token: Address,
}

/// Emits an event when a token is unbound from the contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address that was unbound
#[cfg(not(feature = "certora"))]
fn emit_token_unbound(e: &Env, token: &Address) {
    TokenUnbound { token: token.clone() }.publish(e);
}
