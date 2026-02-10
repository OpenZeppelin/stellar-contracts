//! # Votes Module
//!
//! This module provides utilities for tracking voting power per account with
//! historical checkpoints. It supports delegation (an account can delegate its
//! voting power to another account) and provides historical vote queries at any
//! past timestamp.
//!
//! # Core Concepts
//!
//! - **Voting Units**: The base unit of voting power, typically 1:1 with token
//!   balance
//! - **Delegation**: Accounts can delegate their voting power to another
//!   account (delegatee). **Only delegated voting power counts as votes** while
//!   undelegated voting units are not counted. Self-delegation is required for
//!   an account to use its own voting power.
//! - **Checkpoints**: Historical snapshots of voting power at specific
//!   timestamps
//!
//! # Usage
//!
//! This module is to be integrated into a token contract and is responsible
//! for:
//! - Overriding the transfer method to call `transfer_voting_units` on every
//!   balance change (mint/burn/transfer), as shown in the example below
//! - Exposing delegation functionality to users
//!
//! # Example
//!
//! ```ignore
//! use stellar_governance::votes::{
//!     delegate, get_votes, get_votes_at_checkpoint, transfer_voting_units,
//! };
//!
//! // Override your token contract's transfer to update voting units:
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
    delegate, get_checkpoint, get_delegate, get_total_supply, get_total_supply_at_checkpoint,
    get_votes, get_votes_at_checkpoint, get_voting_units, num_checkpoints, transfer_voting_units,
    Checkpoint, CheckpointType, VotesStorageKey,
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
    /// Returns the current voting power (delegated votes) of an account.
    ///
    /// Returns `0` if the account has no delegated voting power or does not
    /// exist in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query voting power for.
    fn get_votes(e: &Env, account: Address) -> u128 {
        get_votes(e, &account)
    }

    /// Returns the voting power (delegated votes) of an account at a specific
    /// past timestamp.
    ///
    /// Returns `0` if the account had no delegated voting power at the given
    /// timepoint or does not exist in the contract.
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
    fn get_votes_at_checkpoint(e: &Env, account: Address, timepoint: u64) -> u128 {
        get_votes_at_checkpoint(e, &account, timepoint)
    }

    /// Returns the current total supply of voting units.
    ///
    /// This tracks all voting units in circulation (regardless of delegation
    /// status), not just delegated votes.
    ///
    /// Returns `0` if no voting units exist.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_total_supply(e: &Env) -> u128 {
        get_total_supply(e)
    }

    /// Returns the total supply of voting units at a specific past timestamp.
    ///
    /// This tracks all voting units in circulation (regardless of delegation
    /// status), not just delegated votes.
    ///
    /// Returns `0` if there were no voting units at the given timepoint.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `timepoint` - The timestamp to query (must be in the past).
    ///
    /// # Errors
    ///
    /// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
    fn get_total_supply_at_checkpoint(e: &Env, timepoint: u64) -> u128 {
        get_total_supply_at_checkpoint(e, timepoint)
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
    /// * topics - `["delegate_changed", delegator: Address]`
    /// * data - `[from_delegate: Option<Address>, to_delegate: Address]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
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
    /// Attempting to transfer more voting units than available
    InsufficientVotingUnits = 4102,
    /// Attempting to delegate to the same delegate that is already set
    SameDelegate = 4103,
    /// A checkpoint that was expected to exist was not found in storage
    CheckpointNotFound = 4104,
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
    pub previous_votes: u128,
    /// The new voting power
    pub new_votes: u128,
}

/// Emits an event when a delegate's voting power changes.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `delegate` - The delegate whose voting power changed.
/// * `previous_votes` - The previous voting power.
/// * `new_votes` - The new voting power.
pub fn emit_delegate_votes_changed(
    e: &Env,
    delegate: &Address,
    previous_votes: u128,
    new_votes: u128,
) {
    DelegateVotesChanged { delegate: delegate.clone(), previous_votes, new_votes }.publish(e);
}
