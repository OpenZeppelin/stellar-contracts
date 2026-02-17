//! # Counting Storage Module
//!
//! This module provides storage utilities for the Counting module.
//! It defines storage keys and helper functions for managing vote tallies,
//! quorum configuration, and voter tracking.

use soroban_sdk::{contracttype, panic_with_error, Address, BytesN, Env, Symbol};

use crate::counting::{
    emit_quorum_changed, CountingError, COUNTING_EXTEND_AMOUNT, COUNTING_TTL_THRESHOLD,
};

// ################## STORAGE KEYS ##################

/// Storage keys for the Counting module.
#[derive(Clone)]
#[contracttype]
pub enum CountingStorageKey {
    /// The quorum value (minimum participation required).
    Quorum,
    /// Vote tallies for a proposal, indexed by proposal ID.
    ProposalVote(BytesN<32>),
    /// Whether an account has voted on a proposal.
    HasVoted(BytesN<32>, Address),
}

// ################## STORAGE TYPES ##################

/// Vote tallies for a proposal.
#[derive(Clone)]
#[contracttype]
pub struct ProposalVoteCounts {
    /// Total voting power cast against the proposal.
    pub against_votes: u128,
    /// Total voting power cast in favor of the proposal.
    pub for_votes: u128,
    /// Total voting power cast as abstain.
    pub abstain_votes: u128,
}

// ################## CONSTANTS ##################

/// Vote type: Against the proposal.
pub const VOTE_AGAINST: u32 = 0;

/// Vote type: In favor of the proposal.
pub const VOTE_FOR: u32 = 1;

/// Vote type: Abstain from voting for or against.
pub const VOTE_ABSTAIN: u32 = 2;

// ################## QUERY STATE ##################

/// Returns the counting mode identifier.
///
/// For simple counting, this returns `"simple"`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn counting_mode(e: &Env) -> Symbol {
    Symbol::new(e, "simple")
}

/// Returns whether an account has voted on a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `account` - The address to check.
pub fn has_voted(e: &Env, proposal_id: &BytesN<32>, account: &Address) -> bool {
    let key = CountingStorageKey::HasVoted(proposal_id.clone(), account.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, COUNTING_TTL_THRESHOLD, COUNTING_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Returns the quorum value.
///
/// The quorum is the minimum total voting power (for + abstain) that must
/// participate for a proposal to be valid.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`CountingError::QuorumNotSet`] - Occurs if the quorum has not been set.
pub fn get_quorum(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get(&CountingStorageKey::Quorum)
        .unwrap_or_else(|| panic_with_error!(e, CountingError::QuorumNotSet))
}

/// Returns whether the quorum has been reached for a proposal.
///
/// Quorum is reached when the sum of `for` and `abstain` votes meets or
/// exceeds the configured quorum value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
///
/// # Errors
///
/// * [`CountingError::MathOverflow`] - Occurs if participation tally overflows.
/// * also refer to [`get_quorum()`] errors.
pub fn quorum_reached(e: &Env, proposal_id: &BytesN<32>) -> bool {
    let quorum = get_quorum(e);
    let counts = get_proposal_vote_counts(e, proposal_id);

    let Some(participation) = counts.for_votes.checked_add(counts.abstain_votes) else {
        panic_with_error!(e, CountingError::MathOverflow);
    };

    participation >= quorum
}

/// Returns whether the tally has succeeded for a proposal.
///
/// The tally succeeds when the `for` votes strictly exceed the `against` votes.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
pub fn tally_succeeded(e: &Env, proposal_id: &BytesN<32>) -> bool {
    let counts = get_proposal_vote_counts(e, proposal_id);
    counts.for_votes > counts.against_votes
}

/// Returns the vote tallies for a proposal.
///
/// If no tally exists yet, this returns a zero-initialized
/// [`ProposalVoteCounts`].
///
/// Vote tally entries are created lazily on the first recorded vote, not at
/// proposal creation time. This keeps the counting module loosely coupled to
/// governor proposal lifecycle/storage.
///
/// Because of that design, a missing storage entry is interpreted as
/// "no votes cast yet" rather than an error (`panic`) or `Option::None`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
pub fn get_proposal_vote_counts(e: &Env, proposal_id: &BytesN<32>) -> ProposalVoteCounts {
    let key = CountingStorageKey::ProposalVote(proposal_id.clone());
    e.storage()
        .persistent()
        .get::<_, ProposalVoteCounts>(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                COUNTING_TTL_THRESHOLD,
                COUNTING_EXTEND_AMOUNT,
            );
        })
        .unwrap_or(ProposalVoteCounts { against_votes: 0, for_votes: 0, abstain_votes: 0 })
}

// ################## CHANGE STATE ##################

/// Sets the quorum value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `quorum` - The new quorum value.
///
/// # Events
///
/// * topics - `["quorum_changed"]`
/// * data - `[old_quorum: u128, new_quorum: u128]`
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_quorum(e: &Env, quorum: u128) {
    let old_quorum = e.storage().instance().get(&CountingStorageKey::Quorum).unwrap_or(0u128);
    e.storage().instance().set(&CountingStorageKey::Quorum, &quorum);
    emit_quorum_changed(e, old_quorum, quorum);
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
/// * [`CountingError::AlreadyVoted`] - Occurs if the account has already voted
///   on this proposal.
/// * [`CountingError::InvalidVoteType`] - Occurs if the vote type is not 0, 1,
///   or 2.
/// * [`CountingError::MathOverflow`] - Occurs if vote tallying overflows.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn count_vote(
    e: &Env,
    proposal_id: &BytesN<32>,
    account: &Address,
    vote_type: u32,
    weight: u128,
) {
    // Check if the account has already voted
    let voted_key = CountingStorageKey::HasVoted(proposal_id.clone(), account.clone());
    if e.storage().persistent().has(&voted_key) {
        e.storage().persistent().extend_ttl(
            &voted_key,
            COUNTING_TTL_THRESHOLD,
            COUNTING_EXTEND_AMOUNT,
        );

        panic_with_error!(e, CountingError::AlreadyVoted);
    }

    // Get current vote counts
    let mut counts = get_proposal_vote_counts(e, proposal_id);

    // Update vote counts based on vote type
    match vote_type {
        VOTE_AGAINST => {
            let Some(new_against) = counts.against_votes.checked_add(weight) else {
                panic_with_error!(e, CountingError::MathOverflow);
            };
            counts.against_votes = new_against;
        }
        VOTE_FOR => {
            let Some(new_for) = counts.for_votes.checked_add(weight) else {
                panic_with_error!(e, CountingError::MathOverflow);
            };
            counts.for_votes = new_for;
        }
        VOTE_ABSTAIN => {
            let Some(new_abstain) = counts.abstain_votes.checked_add(weight) else {
                panic_with_error!(e, CountingError::MathOverflow);
            };
            counts.abstain_votes = new_abstain;
        }
        _ => panic_with_error!(e, CountingError::InvalidVoteType),
    }

    // Store updated vote counts
    let vote_key = CountingStorageKey::ProposalVote(proposal_id.clone());
    e.storage().persistent().set(&vote_key, &counts);

    // Mark account as having voted
    e.storage().persistent().set(&voted_key, &true);
}
