mod storage;
mod test;
use soroban_sdk::{contracterror, Address, Env, Symbol, Vec};

pub use storage::{bind_token, linked_tokens, unbind_token};

/// Trait for managing token bindings to periphery contracts.
///
/// The `TokenBinder` trait provides a standardized interface for linking tokens
/// to periphery contracts requiring this, such as:
/// - Identity Storage Registry
/// - Compliance contracts
/// - Claim Topics and Issuers Registry
///
/// This binding mechanism allows tokens to be associated with regulatory and
/// compliance infrastructure, enabling features like identity verification,
/// compliance checking, and claim validation.
///
/// # Storage Pattern
///
/// The underlying storage uses an enumerable pattern for efficiency:
/// - Tokens are indexed sequentially (0, 1, 2, ...)
/// - Reverse mapping enables O(1) lookups by token address
/// - Swap-remove pattern maintains compact storage when unbinding
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
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const TOKEN_BINDER_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const TOKEN_BINDER_TTL_THRESHOLD: u32 = TOKEN_BINDER_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Emits an event when a token is bound to the contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address that was bound
///
/// # Events
///
/// * topics - `["token_bound", token: Address]`
/// * data - `[]`
pub fn emit_token_bound(e: &Env, token: &Address) {
    let topics = (Symbol::new(e, "token_bound"), token.clone());
    e.events().publish(topics, ());
}

/// Emits an event when a token is unbound from the contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment
/// * `token` - The token address that was unbound
///
/// # Events
///
/// * topics - `["token_unbound", token: Address]`
/// * data - `[]`
fn emit_token_unbound(e: &Env, token: &Address) {
    let topics = (Symbol::new(e, "token_unbound"), token.clone());
    e.events().publish(topics, ());
}
