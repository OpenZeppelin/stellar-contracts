//! # Governor Queries Storage Module
//!
//! Storage keys and helper functions for reading proposal data: the proposal
//! record, its state, vote tallies and the success criteria.

use soroban_sdk::{
    contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env, Symbol, Val, Vec,
};

use crate::governor::{
    GovernorError, ProposalState, GOVERNOR_EXTEND_AMOUNT, GOVERNOR_TTL_THRESHOLD,
};

// ################## STORAGE KEYS ##################

/// Storage keys for proposal data.
#[derive(Clone)]
#[contracttype]
pub enum ProposalStorageKey {
    /// Proposal data indexed by proposal ID.
    Proposal(BytesN<32>),
    /// Vote tallies for a proposal, indexed by proposal ID.
    ProposalVote(BytesN<32>),
    /// Whether an account has voted on a proposal.
    HasVoted(BytesN<32>, Address),
}

// ################## STORAGE TYPES ##################

/// Core proposal data stored on-chain.
#[derive(Clone)]
#[contracttype]
pub struct ProposalCore {
    /// The address that created the proposal.
    pub proposer: Address,
    /// The ledger at which voting power is snapshotted. Voting opens on
    /// the next ledger (`vote_snapshot + 1`).
    pub vote_snapshot: u32,
    /// The last ledger where voting is active (inclusive).
    pub vote_end: u32,
    /// The current state of the proposal.
    pub state: ProposalState,
}

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

/// Returns the core proposal data.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
pub fn get_proposal_core(e: &Env, proposal_id: &BytesN<32>) -> ProposalCore {
    let key = ProposalStorageKey::Proposal(proposal_id.clone());
    let core: ProposalCore = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::ProposalNotFound));
    e.storage().persistent().extend_ttl(&key, GOVERNOR_TTL_THRESHOLD, GOVERNOR_EXTEND_AMOUNT);
    core
}

/// Returns the current state of a proposal.
///
/// See [`ProposalState`] for the full lifecycle flowchart.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `succeeded` - Whether the vote outcome is a success (quorum reached and
///   tally succeeded). Only used once voting has ended; see
///   [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state)
///   for how the default implementation computes it.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
pub fn get_proposal_state(e: &Env, proposal_id: &BytesN<32>, succeeded: bool) -> ProposalState {
    let core = get_proposal_core(e, proposal_id);
    derive_proposal_state(e, &core, succeeded)
}

/// Returns the snapshot ledger for a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
pub fn get_proposal_snapshot(e: &Env, proposal_id: &BytesN<32>) -> u32 {
    let core = get_proposal_core(e, proposal_id);
    core.vote_snapshot
}

/// Returns the deadline ledger for a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
pub fn get_proposal_deadline(e: &Env, proposal_id: &BytesN<32>) -> u32 {
    let core = get_proposal_core(e, proposal_id);
    core.vote_end
}

/// Returns the proposer of a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
pub fn get_proposal_proposer(e: &Env, proposal_id: &BytesN<32>) -> Address {
    let core = get_proposal_core(e, proposal_id);
    core.proposer
}

/// Returns the vote tallies for a proposal.
///
/// If no tally exists yet, this returns a zero-initialized
/// [`ProposalVoteCounts`].
///
/// Vote tally entries are created lazily on the first recorded vote, not at
/// proposal creation time. This keeps the counting logic loosely coupled to
/// the proposal lifecycle.
///
/// Because of that design, a missing storage entry is interpreted as
/// "no votes cast yet" rather than an error (`panic`) or `Option::None`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
pub fn get_proposal_vote_counts(e: &Env, proposal_id: &BytesN<32>) -> ProposalVoteCounts {
    let key = ProposalStorageKey::ProposalVote(proposal_id.clone());
    e.storage()
        .persistent()
        .get::<_, ProposalVoteCounts>(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                GOVERNOR_TTL_THRESHOLD,
                GOVERNOR_EXTEND_AMOUNT,
            );
        })
        .unwrap_or(ProposalVoteCounts { against_votes: 0, for_votes: 0, abstain_votes: 0 })
}

/// Returns whether an account has voted on a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `account` - The address to check.
pub fn has_voted(e: &Env, proposal_id: &BytesN<32>, account: &Address) -> bool {
    let key = ProposalStorageKey::HasVoted(proposal_id.clone(), account.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, GOVERNOR_TTL_THRESHOLD, GOVERNOR_EXTEND_AMOUNT);
        true
    } else {
        false
    }
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
/// * `quorum` - The quorum threshold, evaluated at the proposal's
///   `vote_snapshot` ledger via
///   [`GovernorSettings::quorum`](crate::governor::GovernorSettings::quorum).
///
/// # Errors
///
/// * [`GovernorError::MathOverflow`] - Occurs if participation tally overflows.
pub fn quorum_reached(e: &Env, proposal_id: &BytesN<32>, quorum: u128) -> bool {
    let counts = get_proposal_vote_counts(e, proposal_id);

    let Some(participation) = counts.for_votes.checked_add(counts.abstain_votes) else {
        panic_with_error!(e, GovernorError::MathOverflow);
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

/// Computes and returns the proposal ID from the proposal parameters.
///
/// The proposal ID is a deterministic keccak256 hash of the XDR-serialized
/// targets, functions, args, and description hash. This allows anyone to
/// compute the ID without storing the full proposal data.
///
/// The `description_hash` is computed as `keccak256(description.to_bytes())`,
/// i.e., a keccak256 hash of the raw UTF-8 bytes of the description string.
/// Off-chain clients can reproduce this by hashing the raw string bytes
/// directly — no XDR encoding is required.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description_hash` - The keccak256 hash of the description's raw bytes.
pub fn hash_proposal(
    e: &Env,
    targets: &Vec<Address>,
    functions: &Vec<Symbol>,
    args: &Vec<Vec<Val>>,
    description_hash: &BytesN<32>,
) -> BytesN<32> {
    // Concatenate all inputs for hashing
    let mut data = Bytes::new(e);
    data.append(&targets.to_xdr(e));
    data.append(&functions.to_xdr(e));
    data.append(&args.to_xdr(e));
    data.append(&Bytes::from_slice(e, description_hash.to_array().as_slice()));

    e.crypto().keccak256(&data).to_bytes()
}

// ################## LOW-LEVEL HELPERS ##################

/// Derives the current state of a proposal.
///
/// Proposal states fall into two categories:
///
/// ## Stored states
///
/// Written to `core.state` by lifecycle functions and returned immediately
/// when present. These represent irreversible transitions that have already
/// occurred:
///
/// * [`ProposalState::Canceled`] — set by [`crate::governor::cancel`].
/// * [`ProposalState::Executed`] — set by [`crate::governor::execute`].
/// * [`ProposalState::Queued`]   — set by [`crate::governor::queue`].
/// * [`ProposalState::Expired`]  — set by extensions (e.g. `TimelockControl`).
///
/// ## Derived states
///
/// Computed on the fly from the current ledger and vote tallies. These are
/// never persisted; `core.state` remains [`ProposalState::Pending`] (the
/// initial value from [`crate::governor::propose`]) throughout the voting
/// lifecycle. This avoids a storage write after every vote while still
/// providing accurate state queries at any point:
///
/// * [`ProposalState::Pending`]   — current ledger is at or before
///   `vote_start`.
/// * [`ProposalState::Active`]    — current ledger is between `vote_start` and
///   `vote_end`. Even if quorum and majority are already met, the proposal
///   remains `Active` until voting closes so that all voters have the
///   opportunity to participate.
/// * [`ProposalState::Succeeded`] — voting ended and `succeeded` is `true`.
/// * [`ProposalState::Defeated`]  — voting ended and `succeeded` is `false`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `core` - The proposal's stored core data.
/// * `succeeded` - Whether the vote outcome is a success. Only used once voting
///   has ended.
fn derive_proposal_state(e: &Env, core: &ProposalCore, succeeded: bool) -> ProposalState {
    // Stored states: return immediately — the transition already happened.
    match core.state {
        ProposalState::Canceled | ProposalState::Executed | ProposalState::Queued => {
            return core.state;
        }
        ProposalState::Expired => {
            return core.state;
        }
        _ => {}
    }

    // Derived states: `core.state` is still `Pending` (its initial value
    // from `propose`), so we determine the actual state from timing and
    // vote tallies.
    let current_ledger = e.ledger().sequence();

    // `vote_snapshot` is the snapshot ledger; voting opens on the next ledger.
    if current_ledger <= core.vote_snapshot {
        return ProposalState::Pending;
    }

    // The proposal stays `Active` until `vote_end` passes, regardless of
    // whether quorum and majority are already met. This ensures all voters
    // have the full voting period to participate.
    if current_ledger <= core.vote_end {
        return ProposalState::Active;
    }

    // Voting has ended — the outcome is decided by the caller.
    if succeeded {
        ProposalState::Succeeded
    } else {
        ProposalState::Defeated
    }
}
