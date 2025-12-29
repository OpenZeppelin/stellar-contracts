//! # Fee Abstraction Module
//!
//! This module provides utilities for implementing fee abstraction in Stellar
//! contracts, allowing users to pay transaction fees in tokens instead of
//! native XLM.
//!
//! # Core Features
//!
//! - **High-level helpers**: wrappers that combine target invocation
//!   (forwarding) and fee collection in the desired order.
//! - **Fee Token Allowlist**: Optional allowlist for accepted fee tokens
//! - **Token Sweeping**: Optional functions to collect accumulated fees
//! - **Fee Validation**: Utilities for validating fee amounts
//!
//! # Usage
//!
//! Prefer the high-level wrappers for integrating fee abstraction:
//! - [`invoke_then_collect_fee`]: invoke target first, then collect fee (see
//!   `examples/fee-forwarder-permissionless` for authorization-tree
//!   implications)
//! - [`collect_fee_then_invoke`]: collect fee first, then invoke target (see
//!   `examples/fee-forwarder-permissioned` for authorization-tree implications)
//!
//! Lower-level helpers ([`auth_user_and_invoke`], [`collect_fee`]) are also
//! exposed for custom composition.
#![no_std]

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, Address, Env, Symbol, Val, Vec};

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL threshold for extending storage entries (in ledgers)
pub const FEE_ABSTRACTION_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL extension amount for storage entries (in ledgers)
pub const FEE_ABSTRACTION_TTL_THRESHOLD: u32 = FEE_ABSTRACTION_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub use crate::storage::{
    auth_user_and_invoke, collect_fee, collect_fee_then_invoke, invoke_then_collect_fee,
    is_allowed_fee_token, is_fee_token_allowlist_enabled, set_allowed_fee_token, sweep_token,
    validate_fee_bounds, FeeAbstractionApproval, FeeAbstractionStorageKey,
};

// ################## ERRORS ##################

/// Errors that can occur in fee abstraction operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FeeAbstractionError {
    /// The fee token is not allowed
    FeeTokenNotAllowed = 5000,
    /// The fee token has been already allowed
    FeeTokenAlreadyAllowed = 5001,
    /// The amount of allowed tokens reached `u32::MAX`
    TokenCountOverflow = 5002,
    /// The fee amount exceeds the maximum allowed
    InvalidFeeBounds = 5003,
    /// No tokens available to sweep
    NoTokensToSweep = 5004,
    /// User address is current contract
    InvalidUser = 5005,
}

// ################## EVENTS ##################

/// Event emitted when a fee token is added or removed from the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTokenAllowlistUpdated {
    #[topic]
    pub token: Address,
    pub allowed: bool,
}

/// Emits an event when a fee token is added or removed from the allowlist.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `token` - The token contract address.
/// * `allowed` - Whether the token is now allowed.
pub fn emit_fee_token_allowlist_updated(e: &Env, token: &Address, allowed: bool) {
    FeeTokenAllowlistUpdated { token: token.clone(), allowed }.publish(e);
}

/// Event emitted when a fee is collected from a user.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    #[topic]
    pub user: Address,
    #[topic]
    pub recipient: Address,
    pub token: Address,
    pub amount: i128,
}

/// Emits an event when a fee is collected from a user.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `user` - The address of the user who paid the fee.
/// * `recipient` - The address that received the fee.
/// * `token` - The token contract address used for payment.
/// * `amount` - The amount of tokens collected.
pub fn emit_fee_collected(
    e: &Env,
    user: &Address,
    recipient: &Address,
    token: &Address,
    amount: i128,
) {
    FeeCollected { user: user.clone(), recipient: recipient.clone(), token: token.clone(), amount }
        .publish(e);
}

/// Event emitted when a call is forwarded to a target contract.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForwardExecuted {
    #[topic]
    pub user: Address,
    #[topic]
    pub target_contract: Address,
    pub target_fn: Symbol,
    pub target_args: Vec<Val>,
}

/// Emits an event when a call is forwarded to a target contract.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `user` - The address of the user who initiated the forward.
/// * `target_contract` - The contract address that was called.
/// * `target_fn` - The function name that was invoked.
/// * `target_args` - The arguments passed to the function.
pub fn emit_forward_executed(
    e: &Env,
    user: &Address,
    target_contract: &Address,
    target_fn: &Symbol,
    target_args: &Vec<Val>,
) {
    ForwardExecuted {
        user: user.clone(),
        target_contract: target_contract.clone(),
        target_fn: target_fn.clone(),
        target_args: target_args.clone(),
    }
    .publish(e);
}

/// Event emitted when tokens are swept from the contract.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokensSwept {
    #[topic]
    pub token: Address,
    #[topic]
    pub recipient: Address,
    pub amount: i128,
}

/// Emits an event when tokens are swept from the contract.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `token` - The token contract address that was swept.
/// * `recipient` - The address that received the tokens.
/// * `amount` - The amount of tokens swept.
pub fn emit_tokens_swept(e: &Env, token: &Address, recipient: &Address, amount: i128) {
    TokensSwept { token: token.clone(), recipient: recipient.clone(), amount }.publish(e);
}
