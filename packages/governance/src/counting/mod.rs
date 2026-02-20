//! # Counting Module
//!
//! This module provides vote counting and quorum logic for on-chain
//! governance.
//!
//! The [`Counting`] trait defines how votes are tallied and how quorum is
//! determined. The default implementation provides **simple counting**:
//!
//! - **Vote types**: Against (0), For (1), Abstain (2)
//! - **Vote success**: `for` votes strictly exceed `against` votes
//! - **Quorum**: Sum of `for` and `abstain` votes meets or exceeds the
//!   configured quorum value (shared across all proposal tallies)
//!
//! # Usage
//!
//! The [`Counting`] trait is expected to be implemented by a Governor contract.
//! The Governor calls [`Counting::count_vote`] when a vote is cast
//! and queries [`Counting::quorum_reached`] and [`Counting::tally_succeeded`]
//! to determine the outcome of a proposal.
//!
//! # Custom Implementations
//!
//! Implementers can override the default trait methods to provide custom
//! counting strategies (e.g., fractional voting, weighted quorum based on
//! total supply, etc.).

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, BytesN, Env, Symbol};

pub use crate::counting::storage::{
    CountingStorageKey, ProposalVoteCounts, VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR,
};

/// Trait for vote counting and quorum logic in governance.
///
/// This trait defines the interface for tallying votes and determining
/// whether a proposal has met the required quorum and vote thresholds.
///
/// The contract that implements this trait is typically also implementing
/// the [`crate::governor::Governor`] trait, which is expected to call these
/// methods during the voting lifecycle.
///
/// # Default Implementation
///
/// The default implementation provides simple counting with three vote
/// types (Against, For, Abstain), simple majority for success, and a
/// fixed quorum value.
#[contracttrait]
pub trait Counting {
    /// Returns a symbol identifying the counting strategy.
    /// This function is expected to be used to display human-readable
    /// information about the counting strategy, for example in UIs.
    ///
    /// For simple counting, this returns `"simple"`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn counting_mode(e: &Env) -> Symbol {
        storage::counting_mode(e)
    }

    /// Returns whether an account has voted on a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    /// * `account` - The address to check.
    fn has_voted(e: &Env, proposal_id: BytesN<32>, account: Address) -> bool {
        storage::has_voted(e, &proposal_id, &account)
    }

    /// Returns the quorum required at the given ledger.
    ///
    /// For simple counting, this returns the configured fixed quorum value
    /// and the `ledger` parameter is ignored. Custom implementations (e.g.,
    /// fractional quorum based on total supply) may use the `ledger`
    /// parameter to compute a dynamic quorum.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `ledger` - The ledger number at which to query the quorum.
    ///
    /// # Errors
    ///
    /// * [`CountingError::QuorumNotSet`] - If the quorum has not been set.
    fn quorum(e: &Env, ledger: u32) -> u128 {
        let _ = ledger;
        storage::get_quorum(e)
    }

    /// Returns whether there was enough participation for a proposal.
    ///
    /// Quorum is reached when the sum of `for` and `abstain` votes meets
    /// or exceeds the configured quorum value.
    ///
    /// Custom implementations (e.g., fractional quorum based on total
    /// supply) may get the `ledger` from the `proposal_id` to compute a dynamic
    /// quorum, hence the `ledger` argument is elided in the trait signature.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`CountingError::QuorumNotSet`] - If the quorum has not been set.
    fn quorum_reached(e: &Env, proposal_id: BytesN<32>) -> bool {
        storage::quorum_reached(e, &proposal_id)
    }

    /// Returns whether the tally has succeeded for a proposal.
    ///
    /// The tally succeeds when the `for` votes strictly exceed the `against`
    /// votes.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    fn tally_succeeded(e: &Env, proposal_id: BytesN<32>) -> bool {
        storage::tally_succeeded(e, &proposal_id)
    }

    /// Records a vote on a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    /// * `account` - The address casting the vote.
    /// * `vote_type` - The type of vote (0 = Against, 1 = For, 2 = Abstain).
    /// * `weight` - The voting power of the voter.
    ///
    /// # Errors
    ///
    /// * [`CountingError::AlreadyVoted`] - If the account has already voted.
    /// * [`CountingError::InvalidVoteType`] - If the vote type is not valid.
    /// * [`CountingError::MathOverflow`] - If vote tallying overflows.
    fn count_vote(
        e: &Env,
        proposal_id: BytesN<32>,
        account: Address,
        vote_type: u32,
        weight: u128,
    ) {
        storage::count_vote(e, &proposal_id, &account, vote_type, weight);
    }
}

// ################## ERRORS ##################

/// Errors that can occur in counting operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CountingError {
    /// The account has already voted on this proposal.
    AlreadyVoted = 4200,
    /// The vote type is invalid (must be 0, 1, or 2).
    InvalidVoteType = 4201,
    /// The quorum has not been set.
    QuorumNotSet = 4202,
    /// Arithmetic overflow occurred.
    MathOverflow = 4203,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for storage entries (in ledgers)
pub const COUNTING_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL threshold for extending storage entries (in ledgers)
pub const COUNTING_TTL_THRESHOLD: u32 = COUNTING_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when the quorum value is changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuorumChanged {
    pub old_quorum: u128,
    pub new_quorum: u128,
}

/// Emits an event when the quorum value is changed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `old_quorum` - The previous quorum value.
/// * `new_quorum` - The new quorum value.
pub fn emit_quorum_changed(e: &Env, old_quorum: u128, new_quorum: u128) {
    QuorumChanged { old_quorum, new_quorum }.publish(e);
}
