//! # Governor Queries Module
//!
//! Read-only information about individual proposals, including the state of
//! a proposal and whether it passed its vote. See [`GovernorQueries`].

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracttrait, Address, BytesN, Env, Symbol, Val, Vec};

use crate::governor::{GovernorSettings, ProposalState};

/// Read-only information about individual proposals, including how the outcome
/// of a vote is decided.
///
/// [`GovernorQueries::proposal_state`] is the single source of truth for the
/// state of a proposal: every state check in [`crate::governor::Governor`]
/// (voting, queueing and execution) acts on what it returns. Overriding it, or
/// [`GovernorQueries::proposal_succeeded`] which decides whether a proposal
/// passed, changes the outcome everywhere.
#[contracttrait]
pub trait GovernorQueries: GovernorSettings {
    /// Returns the current state of a proposal.
    ///
    /// This is the state that [`crate::governor::Governor::cast_vote`],
    /// [`crate::governor::Governor::queue`] and
    /// [`crate::governor::storage::execute`] act on. The default
    /// implementation returns a recorded state (`Queued`, `Executed`,
    /// `Canceled` or `Expired`) if one exists, and otherwise derives the
    /// state from the current ledger and the votes. Once voting has
    /// ended, the proposal is `Succeeded` if
    /// [`GovernorQueries::proposal_succeeded`] returns `true`, and `Defeated`
    /// otherwise.
    ///
    /// # Overriding
    ///
    /// Since every state check acts on this method, an override changes the
    /// whole lifecycle, not only what is reported. An override must keep
    /// returning the recorded states unchanged. For example, reporting an
    /// `Executed` proposal as `Succeeded` would allow it to be executed again.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::ProposalNotFound`] - If the proposal
    ///   does not exist.
    /// * refer to [`GovernorQueries::proposal_succeeded`] errors.
    fn proposal_state(e: &Env, proposal_id: BytesN<32>) -> ProposalState {
        let succeeded = Self::proposal_succeeded(e, proposal_id.clone());
        storage::get_proposal_state(e, &proposal_id, succeeded)
    }

    /// Returns whether a proposal passed its vote.
    ///
    /// The default implementation requires both of the following:
    ///
    /// * **Quorum**: the `for` and `abstain` votes reach
    ///   [`GovernorSettings::quorum`] at the proposal's `vote_snapshot` ledger.
    /// * **Tally**: the `for` votes strictly exceed the `against` votes.
    ///
    /// An override can change either criterion, e.g. to count `absent` votes
    /// toward the quorum, or to require "at least 60% of the `for` and
    /// `against` votes to be `for`". An override replaces both criteria, so it
    /// must check the quorum as well to keep requiring it:
    ///
    /// ```ignore
    /// fn proposal_succeeded(e: &Env, proposal_id: BytesN<32>) -> bool {
    ///     let quorum = Self::quorum(e, governor::get_proposal_snapshot(e, &proposal_id));
    ///     let counts = governor::get_proposal_vote_counts(e, &proposal_id);
    ///     governor::quorum_reached(e, &proposal_id, quorum)
    ///         && counts.for_votes * 100 >= (counts.for_votes + counts.against_votes) * 60
    /// }
    /// ```
    ///
    /// The default [`GovernorQueries::proposal_state`] calls this method on
    /// every call, but its result only matters once voting has ended. When the
    /// rule is overridden, [`GovernorSettings::counting_mode`] should be
    /// updated to describe it.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::ProposalNotFound`] - If the proposal
    ///   does not exist.
    /// * [`crate::governor::GovernorError::QuorumNotSet`] - If no quorum
    ///   checkpoint exists at or before the proposal's `vote_snapshot` ledger.
    /// * [`crate::governor::GovernorError::MathOverflow`] - If the sum of `for`
    ///   and `abstain` votes overflows.
    fn proposal_succeeded(e: &Env, proposal_id: BytesN<32>) -> bool {
        let quorum = Self::quorum(e, storage::get_proposal_snapshot(e, &proposal_id));
        storage::quorum_reached(e, &proposal_id, quorum)
            && storage::tally_succeeded(e, &proposal_id)
    }

    /// Returns the ledger number at which voting power is retrieved for a
    /// proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::ProposalNotFound`] - If the proposal
    ///   does not exist.
    fn proposal_snapshot(e: &Env, proposal_id: BytesN<32>) -> u32 {
        storage::get_proposal_snapshot(e, &proposal_id)
    }

    /// Returns the ledger number at which voting ends for a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::ProposalNotFound`] - If the proposal
    ///   does not exist.
    fn proposal_deadline(e: &Env, proposal_id: BytesN<32>) -> u32 {
        storage::get_proposal_deadline(e, &proposal_id)
    }

    /// Returns the address of the proposer for a given proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::ProposalNotFound`] - If the proposal
    ///   does not exist.
    fn proposal_proposer(e: &Env, proposal_id: BytesN<32>) -> Address {
        storage::get_proposal_proposer(e, &proposal_id)
    }

    /// Returns the proposal ID computed from the proposal details.
    ///
    /// The proposal ID is a deterministic keccak256 hash of the XDR-serialized
    /// targets, functions, args, and description hash. This allows anyone to
    /// compute the ID without storing the full proposal data.
    ///
    /// The `description_hash` is computed as
    /// `keccak256(description.to_bytes())`, i.e., a keccak256 hash of the raw
    /// UTF-8 bytes of the description string. Off-chain clients can reproduce
    /// this by hashing the raw string bytes directly — no XDR encoding is
    /// required.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `targets` - The addresses of contracts to call.
    /// * `functions` - The function names to invoke on each target.
    /// * `args` - The arguments for each function call.
    /// * `description_hash` - The keccak256 hash of the description's raw
    ///   bytes.
    fn get_proposal_id(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
    ) -> BytesN<32> {
        storage::hash_proposal(e, &targets, &functions, &args, &description_hash)
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
}
