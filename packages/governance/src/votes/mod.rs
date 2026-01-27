//! # Votes Module
//!
//! The module tracks voting power per account with historical checkpoints,
//! supports delegation (an account can delegate its voting power to another
//! account), and provides historical vote queries at any past timestamp.
//!
//! # Core Concepts
//!
//! - **Voting Units**: The base unit of voting power, typically 1:1 with token
//!   balance
//! - **Delegation**: Accounts can delegate their voting power to another
//!   account (delegatee)
//! - **Checkpoints**: Historical snapshots of voting power at specific
//!   timestamps
//!
//! # Design
//!
//! This module follows the design of OpenZeppelin's Solidity `Votes.sol`:
//! - Voting units must be explicitly delegated to count as votes
//! - Self-delegation is required for an account to use its own voting power
//! - Historical vote queries use binary search over checkpoints
//!
//! # Usage
//!
//! This module provides storage functions that can be integrated into a token
//! contract. The contract is responsible for:
//! - Calling `transfer_voting_units` on every balance change
//!   (mint/burn/transfer)
//! - Exposing delegation functionality to users
//!
//! # Example
//!
//! ```ignore
//! use stellar_governance::votes::{
//!     delegate, get_votes, get_past_votes, transfer_voting_units,
//! };
//!
//! // In your token contract transfer:
//! pub fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
//!     // ... perform transfer logic ...
//!     transfer_voting_units(e, Some(&from), Some(&to), amount as u128);
//! }
//!
//! // Expose delegation:
//! pub fn delegate(e: &Env, account: Address, delegatee: Address) {
//!     votes::delegate(e, &account, &delegatee);
//! }
//! ```

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, Env};

pub use crate::votes::storage::{
    delegate, get_delegate, get_past_total_supply, get_past_votes, get_total_supply, get_votes,
    get_voting_units, num_checkpoints, transfer_voting_units, Checkpoint, VotesStorageKey,
};

/// Trait for contracts that support vote tracking with delegation.
///
/// This trait defines the interface for vote tracking functionality.
/// Contracts implementing this trait can be used in governance systems
/// that require historical vote queries and delegation.
///
/// # Implementation Notes
///
/// The implementing contract must:
/// - Call `transfer_voting_units` on every balance change
/// - Expose `delegate` functionality to users
#[contracttrait]
pub trait Votes {
    /// Returns the current voting power of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query voting power for.
    fn get_votes(e: &Env, account: Address) -> u128 {
        get_votes(e, &account)
    }

    /// Returns the voting power of an account at a specific past timestamp.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query voting power for.
    /// * `timepoint` - The timestamp to query (must be in the past).
    ///
    /// # Errors
    ///
    /// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
    fn get_past_votes(e: &Env, account: Address, timepoint: u64) -> u128 {
        get_past_votes(e, &account, timepoint)
    }

    /// Returns the total supply of voting units at a specific past timestamp.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `timepoint` - The timestamp to query (must be in the past).
    ///
    /// # Errors
    ///
    /// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
    fn get_past_total_supply(e: &Env, timepoint: u64) -> u128 {
        get_past_total_supply(e, timepoint)
    }

    /// Returns the current delegate for an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query the delegate for.
    ///
    /// # Returns
    ///
    /// * `Some(Address)` - The delegate address if delegation is set.
    /// * `None` - If the account has not delegated.
    fn get_delegate(e: &Env, account: Address) -> Option<Address> {
        get_delegate(e, &account)
    }

    /// Delegates voting power from `account` to `delegatee`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The account delegating its voting power.
    /// * `delegatee` - The account receiving the delegated voting power.
    ///
    /// # Events
    ///
    /// * [`DelegateChanged`] - Emitted when delegation changes.
    /// * [`DelegateVotesChanged`] - Emitted for both old and new delegates if
    ///   their voting power changes.
    ///
    /// # Notes
    ///
    /// Authorization for `account` is required.
    fn delegate(e: &Env, account: Address, delegatee: Address) {
        delegate(e, &account, &delegatee);
    }
}
// ################## ERRORS ##################

/// Errors that can occur in votes operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VotesError {
    /// The timepoint is in the future
    FutureLookup = 4100,
    /// Arithmetic overflow occurred
    MathOverflow = 4101,
    /// Try to transfer more than available
    InsufficientVotingUnits = 4102,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for storage entries (in ledgers)
pub const VOTES_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL threshold for extending storage entries (in ledgers)
pub const VOTES_TTL_THRESHOLD: u32 = VOTES_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when an account changes its delegate.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegateChanged {
    /// The account that changed its delegate
    #[topic]
    pub delegator: Address,
    /// The previous delegate (if any)
    pub from_delegate: Option<Address>,
    /// The new delegate
    pub to_delegate: Address,
}

/// Emits an event when an account changes its delegate.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `delegator` - The account that changed its delegate.
/// * `from_delegate` - The previous delegate (if any).
/// * `to_delegate` - The new delegate.
pub fn emit_delegate_changed(
    e: &Env,
    delegator: &Address,
    from_delegate: Option<Address>,
    to_delegate: &Address,
) {
    DelegateChanged {
        delegator: delegator.clone(),
        from_delegate,
        to_delegate: to_delegate.clone(),
    }
    .publish(e);
}

/// Event emitted when a delegate's voting power changes.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegateVotesChanged {
    /// The delegate whose voting power changed
    #[topic]
    pub delegate: Address,
    /// The previous voting power
    pub old_votes: u128,
    /// The new voting power
    pub new_votes: u128,
}

/// Emits an event when a delegate's voting power changes.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `delegate` - The delegate whose voting power changed.
/// * `old_votes` - The previous voting power.
/// * `new_votes` - The new voting power.
pub fn emit_delegate_votes_changed(e: &Env, delegate: &Address, old_votes: u128, new_votes: u128) {
    DelegateVotesChanged { delegate: delegate.clone(), old_votes, new_votes }.publish(e);
}
