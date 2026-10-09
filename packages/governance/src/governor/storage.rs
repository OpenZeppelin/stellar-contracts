//! # Governor Storage Module
//!
//! Helper functions for the proposal lifecycle: creating proposals, voting,
//! queueing, executing and cancelling. Proposal data is stored under the keys
//! defined in [`crate::governor::queries::storage`], and the configuration
//! under the keys in [`crate::governor::settings::storage`].

use soroban_sdk::{panic_with_error, Address, BytesN, Env, String, Symbol, Val, Vec};

use crate::{
    governor::{
        emit_proposal_cancelled, emit_proposal_created, emit_proposal_executed,
        emit_proposal_queued, emit_vote_cast,
        queries::storage::{
            get_proposal_core, get_proposal_snapshot, get_proposal_vote_counts, hash_proposal,
            ProposalCore, ProposalStorageKey, VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR,
        },
        settings::storage::get_token_contract,
        GovernorError, ProposalState, MAX_DESCRIPTION_LENGTH,
    },
    votes::VotesClient,
};

// ################## STORAGE TYPES ##################

/// Configuration values that [`propose`] enforces when creating a proposal.
///
/// The default [`Governor::propose`](crate::governor::Governor::propose)
/// fills these from the trait methods
/// ([`GovernorSettings::proposal_threshold`](crate::governor::GovernorSettings::proposal_threshold),
/// [`GovernorSettings::voting_delay`](crate::governor::GovernorSettings::voting_delay) and
/// [`GovernorSettings::voting_period`](crate::governor::GovernorSettings::voting_period)),
/// so overrides of those methods are what gets enforced.
///
/// This type is only passed between functions; it is never stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProposalSettings {
    /// Minimum voting power the proposer must hold at the previous ledger.
    pub threshold: u128,
    /// Number of ledgers between proposal creation and the vote snapshot.
    pub voting_delay: u32,
    /// Number of ledgers during which voting is open.
    pub voting_period: u32,
}

// ################## CHANGE STATE ##################

/// Creates a new proposal and returns its unique identifier (proposal ID).
///
/// Fetches the proposer's voting power from the token contract at the
/// previous ledger (snapshot) to prevent flash-loan-based proposals.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description` - A description of the proposal.
/// * `proposer` - The address creating the proposal.
/// * `settings` - The proposal threshold, voting delay and voting period to
///   enforce. See [`ProposalSettings`].
///
/// # Errors
///
/// * [`GovernorError::EmptyProposal`] - Occurs if the proposal contains no
///   actions.
/// * [`GovernorError::InvalidProposalLength`] - Occurs if targets, functions,
///   and args vectors have different lengths.
/// * [`GovernorError::ProposalAlreadyExists`] - Occurs if a proposal with the
///   same parameters already exists.
/// * [`GovernorError::InsufficientProposerVotes`] - Occurs if the proposer
///   lacks sufficient voting power.
/// * [`GovernorError::MathOverflow`] - Occurs if voting schedule calculation
///   overflows.
/// * refer to [`get_token_contract()`] errors.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn propose(
    e: &Env,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description: String,
    proposer: &Address,
    settings: &ProposalSettings,
) -> BytesN<32> {
    // Validate proposal length
    let targets_len = targets.len();
    if targets_len == 0 {
        panic_with_error!(e, GovernorError::EmptyProposal);
    }
    if targets_len != functions.len() || targets_len != args.len() {
        panic_with_error!(e, GovernorError::InvalidProposalLength);
    }

    // Validate description length to prevent oversized events.
    if description.len() > MAX_DESCRIPTION_LENGTH {
        panic_with_error!(e, GovernorError::DescriptionTooLong);
    }

    // Use previous ledger to prevent flash loan based proposals
    let snapshot = e.ledger().sequence() - 1;
    let proposer_votes = get_voting_power(e, proposer, snapshot);

    // Check proposer has sufficient voting power
    if proposer_votes < settings.threshold {
        panic_with_error!(e, GovernorError::InsufficientProposerVotes);
    }

    let current_ledger = e.ledger().sequence();

    // Compute proposal ID
    let description_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();
    let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);

    // Check proposal doesn't already exist
    if e.storage().persistent().has(&ProposalStorageKey::Proposal(proposal_id.clone())) {
        panic_with_error!(e, GovernorError::ProposalAlreadyExists);
    }

    // Calculate voting schedule
    let Some(vote_snapshot) = current_ledger.checked_add(settings.voting_delay) else {
        panic_with_error!(e, GovernorError::MathOverflow);
    };
    let Some(vote_end) = vote_snapshot.checked_add(settings.voting_period) else {
        panic_with_error!(e, GovernorError::MathOverflow);
    };

    // Store proposal
    let proposal = ProposalCore {
        proposer: proposer.clone(),
        vote_snapshot,
        vote_end,
        state: ProposalState::Pending,
    };
    e.storage().persistent().set(&ProposalStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    emit_proposal_created(
        e,
        &proposal_id,
        proposer,
        &targets,
        &functions,
        &args,
        vote_snapshot,
        vote_end,
        &description,
    );

    proposal_id
}

/// Casts a vote on a proposal and returns the voter's voting power.
///
/// This is the high-level vote flow: it verifies the proposal is active,
/// fetches the voter's voting power from the token contract at the proposal
/// snapshot, records the vote, and emits a
/// [`VoteCast`](crate::governor::VoteCast) event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `vote_type` - The type of vote (0 = Against, 1 = For, 2 = Abstain).
/// * `reason` - An optional explanation for the vote.
/// * `voter` - The address casting the vote.
/// * `state` - The proposal's current state, as returned by
///   [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state).
///
/// # Errors
///
/// * [`GovernorError::ProposalNotActive`] - If the proposal is not active.
/// * [`GovernorError::AlreadyVoted`] - If the voter has already voted.
/// * [`GovernorError::InvalidVoteType`] - If the vote type is invalid.
/// * [`GovernorError::MathOverflow`] - If vote tallying overflows.
/// * refer to [`get_proposal_core()`] errors.
/// * refer to [`get_token_contract()`] errors.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
///
/// The `state` argument is trusted as given. It must come from
/// [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state);
/// passing any other value (e.g. a hardcoded `Active`) bypasses the voting
/// window.
pub fn cast_vote(
    e: &Env,
    proposal_id: &BytesN<32>,
    vote_type: u32,
    reason: &String,
    voter: &Address,
    state: ProposalState,
) -> u128 {
    if state != ProposalState::Active {
        panic_with_error!(e, GovernorError::ProposalNotActive);
    }
    let snapshot = get_proposal_snapshot(e, proposal_id);
    let voter_weight = get_voting_power(e, voter, snapshot);
    count_vote(e, proposal_id, voter, vote_type, voter_weight);
    emit_vote_cast(e, voter, proposal_id, vote_type, voter_weight, reason);
    voter_weight
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
/// * [`GovernorError::AlreadyVoted`] - Occurs if the account has already voted
///   on this proposal.
/// * [`GovernorError::InvalidVoteType`] - Occurs if the vote type is not 0, 1,
///   or 2.
/// * [`GovernorError::MathOverflow`] - Occurs if vote tallying overflows.
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
    let voted_key = ProposalStorageKey::HasVoted(proposal_id.clone(), account.clone());
    if e.storage().persistent().has(&voted_key) {
        panic_with_error!(e, GovernorError::AlreadyVoted);
    }

    // Get current vote counts
    let mut counts = get_proposal_vote_counts(e, proposal_id);

    // Update vote counts based on vote type
    match vote_type {
        VOTE_AGAINST => {
            let Some(new_against) = counts.against_votes.checked_add(weight) else {
                panic_with_error!(e, GovernorError::MathOverflow);
            };
            counts.against_votes = new_against;
        }
        VOTE_FOR => {
            let Some(new_for) = counts.for_votes.checked_add(weight) else {
                panic_with_error!(e, GovernorError::MathOverflow);
            };
            counts.for_votes = new_for;
        }
        VOTE_ABSTAIN => {
            let Some(new_abstain) = counts.abstain_votes.checked_add(weight) else {
                panic_with_error!(e, GovernorError::MathOverflow);
            };
            counts.abstain_votes = new_abstain;
        }
        _ => panic_with_error!(e, GovernorError::InvalidVoteType),
    }

    // Store updated vote counts
    let vote_key = ProposalStorageKey::ProposalVote(proposal_id.clone());
    e.storage().persistent().set(&vote_key, &counts);

    // Mark account as having voted
    e.storage().persistent().set(&voted_key, &true);
}

/// Queues a succeeded proposal for execution and returns its unique identifier
/// (proposal ID).
///
/// Transitions the proposal from [`ProposalState::Succeeded`] to
/// [`ProposalState::Queued`]. The `eta` (estimated time of arrival) is
/// emitted in the event for off-chain consumers but is **not enforced** by
/// this function or by [`execute`]. Enforcement of the execution delay is
/// the responsibility of the integration layer (e.g., a timelock contract).
/// The `eta` is typically computed by the caller as
/// `current_ledger + timelock_delay`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description_hash` - The hash of the proposal description.
/// * `eta` - The estimated ledger sequence for execution. Emitted in the event
///   only; not stored or enforced by the governor.
/// * `state` - The proposal's current state, as returned by
///   [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state).
///
/// # Errors
///
/// * [`GovernorError::ProposalNotSuccessful`] - Occurs if the proposal is not
///   in the `Succeeded` state.
/// * refer to [`get_proposal_core()`] errors.
///
/// # Events
///
/// * topics - `["proposal_queued", proposal_id: BytesN<32>]`
/// * data - `[eta: u32]`
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
///
/// The `state` argument is trusted as given. It must come from
/// [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state);
/// passing any other value (e.g. a hardcoded `Succeeded`) bypasses the
/// outcome of the vote.
pub fn queue(
    e: &Env,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description_hash: &BytesN<32>,
    eta: u32,
    state: ProposalState,
) -> BytesN<32> {
    let proposal_id = hash_proposal(e, &targets, &functions, &args, description_hash);
    let mut proposal = get_proposal_core(e, &proposal_id);
    if state != ProposalState::Succeeded {
        panic_with_error!(e, GovernorError::ProposalNotSuccessful);
    }

    proposal.state = ProposalState::Queued;
    e.storage().persistent().set(&ProposalStorageKey::Proposal(proposal_id.clone()), &proposal);

    emit_proposal_queued(e, &proposal_id, eta);

    proposal_id
}

/// Executes a successful proposal and returns its unique identifier (proposal
/// ID).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description_hash` - The hash of the proposal description.
/// * `queue_enabled` - Whether queueing is enabled (i.e., whether the proposal
///   must be in the `Queued` state to execute).
/// * `state` - The proposal's current state, as returned by
///   [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state).
///
/// # Errors
///
/// * [`GovernorError::ProposalNotSuccessful`] - Occurs if the proposal has not
///   succeeded.
/// * [`GovernorError::ProposalAlreadyExecuted`] - Occurs if the proposal has
///   already been executed.
/// * [`GovernorError::ProposalNotQueued`] - Occurs if queueing is enabled and
///   the proposal is not in the `Queued` state.
/// * refer to [`get_proposal_core()`] errors.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
///
/// The `state` argument is trusted as given. It must come from
/// [`GovernorQueries::proposal_state`](crate::governor::GovernorQueries::proposal_state);
/// passing any other value (e.g. a hardcoded `Succeeded`) bypasses the
/// outcome of the vote.
pub fn execute(
    e: &Env,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description_hash: &BytesN<32>,
    queue_enabled: bool,
    state: ProposalState,
) -> BytesN<32> {
    let proposal_id = hash_proposal(e, &targets, &functions, &args, description_hash);

    // Get proposal and verify it exists
    let mut proposal = get_proposal_core(e, &proposal_id);

    // Check proposal state
    if state == ProposalState::Executed {
        panic_with_error!(e, GovernorError::ProposalAlreadyExecuted);
    }
    if queue_enabled {
        if state != ProposalState::Queued {
            panic_with_error!(e, GovernorError::ProposalNotQueued);
        }
    } else if state != ProposalState::Succeeded {
        panic_with_error!(e, GovernorError::ProposalNotSuccessful);
    }

    // Execute each action
    //
    // `propose()` ensures the proposals in the storage are in the
    // correct state, no further checks on the proposal integrity are needed.
    // It should be safe to use `get_unchecked` here.
    for i in 0..targets.len() {
        let target = targets.get_unchecked(i);
        let function = functions.get_unchecked(i);
        let func_args = args.get_unchecked(i);
        e.invoke_contract::<Val>(&target, &function, func_args);
    }

    // Mark as executed
    proposal.state = ProposalState::Executed;
    e.storage().persistent().set(&ProposalStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    emit_proposal_executed(e, &proposal_id);

    proposal_id
}

/// Cancels a proposal and returns its unique identifier (proposal ID).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description_hash` - The hash of the proposal description.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotCancellable`] - Occurs if the proposal is in a
///   non-cancellable state (`Canceled`, `Expired`, or `Executed`).
/// * refer to [`get_proposal_core()`] errors.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
///
/// # Note
///
/// This function only updates the governor-level proposal state. If the
/// proposal has already been queued in an external timelock, the
/// corresponding timelock operation must be cancelled separately (e.g. via
/// [`crate::timelock::cancel_operation`])
/// to prevent it from remaining executable through the timelock directly.
pub fn cancel(
    e: &Env,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description_hash: &BytesN<32>,
) -> BytesN<32> {
    let proposal_id = hash_proposal(e, &targets, &functions, &args, description_hash);

    // Get proposal and verify it exists
    let mut proposal = get_proposal_core(e, &proposal_id);

    // Blacklist non-cancellable explicit states.
    // These are always stored directly in `core.state`, so no need to derive
    // the full proposal state (which would also require a vote-count read).
    match proposal.state {
        ProposalState::Canceled | ProposalState::Expired | ProposalState::Executed => {
            panic_with_error!(e, GovernorError::ProposalNotCancellable)
        }
        _ => {}
    }

    // Mark as cancelled
    proposal.state = ProposalState::Canceled;
    e.storage().persistent().set(&ProposalStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    emit_proposal_cancelled(e, &proposal_id);

    proposal_id
}

// ################## LOW-LEVEL HELPERS ##################

/// Fetches the voting power of an account at a specific ledger sequence
/// number from the token contract via a cross-contract call to
/// `get_votes_at_checkpoint`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to query voting power for.
/// * `ledger` - The ledger sequence number to query.
///
/// # Errors
///
/// * refer to [`get_token_contract()`] errors.
fn get_voting_power(e: &Env, account: &Address, ledger: u32) -> u128 {
    let token = get_token_contract(e);
    VotesClient::new(e, &token).get_votes_at_checkpoint(&account.clone(), &ledger)
}
