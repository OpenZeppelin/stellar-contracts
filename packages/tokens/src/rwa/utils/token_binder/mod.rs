mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, Env, Vec};
pub use storage::{
    bind_token, bind_tokens, is_token_bound, linked_token_count, linked_tokens, unbind_token,
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
/// - All bound token addresses live in a single `Vec<Address>` ledger entry
/// - Swap-remove pattern keeps the list compact when unbinding, so the list
///   order is not stable across unbinds
///
/// Note that the storage module also exposes a batch binding helper
/// `bind_tokens(e, tokens)` which is not part of this trait, so that client
/// contracts can decide how to expose batch semantics in their own interfaces.
///
/// Implementation notes:
/// - All token addresses live in a single `Vec<Address>` ledger entry. At most
///   [`MAX_TOKENS`] = 100 tokens can be bound to a single contract: the cap is
///   deliberately small enough that a sweep making one cross-contract call per
///   bound token fits in a single transaction, and it keeps the whole list a
///   few kilobytes, far below the per-entry size limit. Refer to [`MAX_TOKENS`]
///   for the sizing rationale.
/// - With Protocol 23, reading live Soroban state is inexpensive. Lookups are
///   therefore cheap, and storage remains simple with no reverse mapping;
///   membership checks (`is_token_bound()`) linearly scan the list.
#[contracttrait]
pub trait TokenBinder {
    /// Returns all currently bound token addresses.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment
    fn linked_tokens(e: &Env) -> Vec<Address> {
        storage::linked_tokens(e)
    }

    /// Binds a token to this contract's periphery services.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment
    /// * `token` - The token address to bind
    /// * `operator` - The address authorizing this operation
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`bind_token`] for the
    /// implementation.
    fn bind_token(e: &Env, token: Address, operator: Address);

    /// Unbinds a token from this contract's periphery services.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment
    /// * `token` - The token address to unbind
    /// * `operator` - The address authorizing this operation
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`unbind_token`] for the
    /// implementation.
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
    /// The batch contains duplicates.
    BindBatchDuplicates = 334,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const TOKEN_BINDER_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const TOKEN_BINDER_TTL_THRESHOLD: u32 = TOKEN_BINDER_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Max. number of Token addresses.
///
/// The cap is sized so that operations sweeping every bound token with one
/// cross-contract call each (such as the identity registry's zero-balance
/// check on `remove_identity`) fit within a single transaction. The binding
/// constraint on current Mainnet is the 400 footprint entries allowed per
/// transaction: each swept token touches up to three distinct entries
/// (contract instance, contract code, one data entry), so 100 tokens consume
/// at most ~300 entries and leave room for the caller's own footprint. CPU
/// stays well within budget at this size. Current limits are listed at
/// <https://lab.stellar.org/network-limits>.
///
/// The cap also keeps the whole token list viable as a single ledger entry:
/// 100 addresses take a few kilobytes, far below the 64 KB per-entry limit.
pub const MAX_TOKENS: u32 = 100;

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
fn emit_token_unbound(e: &Env, token: &Address) {
    TokenUnbound { token: token.clone() }.publish(e);
}
