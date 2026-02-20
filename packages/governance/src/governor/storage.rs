//! # Governor Storage Module
//!
//! This module provides storage utilities for the Governor contract.
//! It defines storage keys and helper functions for managing proposal state,
//! votes, and configuration parameters.

use soroban_sdk::{
    contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env, IntoVal, String,
    Symbol, Val, Vec,
};

use crate::governor::{
    emit_proposal_created, emit_vote_cast, GovernorError, ProposalState, GOVERNOR_EXTEND_AMOUNT,
    GOVERNOR_TTL_THRESHOLD,
};

// ################## STORAGE KEYS ##################

/// Storage keys for the Governor contract.
#[derive(Clone)]
#[contracttype]
pub enum GovernorStorageKey {
    /// The name of the governor.
    Name,
    /// The version of the governor contract.
    Version,
    /// The voting delay in ledgers.
    VotingDelay,
    /// The voting period in ledgers.
    VotingPeriod,
    /// Minimum voting power required to propose.
    ProposalThreshold,
    /// Proposal data indexed by proposal ID.
    Proposal(BytesN<32>),
    /// The quorum value (minimum participation required).
    Quorum,
    /// Vote tallies for a proposal, indexed by proposal ID.
    ProposalVote(BytesN<32>),
    /// Whether an account has voted on a proposal.
    HasVoted(BytesN<32>, Address),
    /// The address of the token contract that implements the Votes trait.
    TokenContract,
}

// ################## STORAGE TYPES ##################

/// Core proposal data stored on-chain.
#[derive(Clone)]
#[contracttype]
pub struct ProposalCore {
    /// The address that created the proposal.
    pub proposer: Address,
    /// The ledger number when voting starts.
    pub vote_start: u32,
    /// The ledger number when voting ends.
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

// ################## QUERY_STATE ##################

/// Returns the name of the governor.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::NameNotSet`] - Occurs if the name has not been set.
pub fn get_name(e: &Env) -> String {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::Name)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::NameNotSet))
}

/// Returns the version of the governor contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::VersionNotSet`] - Occurs if the version has not been set.
pub fn get_version(e: &Env) -> String {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::Version)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VersionNotSet))
}

/// Returns the proposal threshold.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::ProposalThresholdNotSet`] - Occurs if the proposal
///   threshold has not been set.
pub fn get_proposal_threshold(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::ProposalThreshold)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::ProposalThresholdNotSet))
}

/// Returns the voting delay in ledgers.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::VotingDelayNotSet`] - Occurs if the voting delay has not
///   been set.
pub fn get_voting_delay(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::VotingDelay)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotingDelayNotSet))
}

/// Returns the voting period in ledgers.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::VotingPeriodNotSet`] - Occurs if the voting period has
///   not been set.
pub fn get_voting_period(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::VotingPeriod)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotingPeriodNotSet))
}

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
    let key = GovernorStorageKey::Proposal(proposal_id.clone());
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
///
/// # Errors
///
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
///
/// # Note
///
/// Queue logic is expected to be implemented by an extension, so this
/// function does not handle the `Queued` and `Expired` states.
pub fn get_proposal_state(e: &Env, proposal_id: &BytesN<32>) -> ProposalState {
    let core = get_proposal_core(e, proposal_id);
    derive_proposal_state(e, proposal_id, &core)
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
    core.vote_start
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

/// Returns the address of the token contract that implements the Votes trait.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::TokenContractNotSet`] - Occurs if the token contract has
///   not been set.
pub fn get_token_contract(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::TokenContract)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::TokenContractNotSet))
}

// ################## CHANGE STATE ##################

/// Sets the name of the governor.
///
/// The name is not validated here. It is the responsibility of the
/// implementer to ensure that the name is appropriate.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The name to set.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_name(e: &Env, name: String) {
    e.storage().instance().set(&GovernorStorageKey::Name, &name);
}

/// Sets the version of the governor contract.
///
/// The version is not validated here. It is the responsibility of the
/// implementer to ensure that the version string is appropriate.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `version` - The version string to set.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_version(e: &Env, version: String) {
    e.storage().instance().set(&GovernorStorageKey::Version, &version);
}

/// Sets the proposal threshold.
///
/// The threshold value is not validated here. It is the responsibility of
/// the implementer to ensure that the threshold is reasonable for the
/// governance use case (e.g., not so high that no one can propose).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `threshold` - The minimum voting power required to create a proposal.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_proposal_threshold(e: &Env, threshold: u128) {
    e.storage().instance().set(&GovernorStorageKey::ProposalThreshold, &threshold);
}

/// Sets the voting delay.
///
/// The delay value is not validated here. It is the responsibility of
/// the implementer to ensure that the delay is appropriate (e.g., enough
/// time for token holders to prepare, but not so long that governance
/// becomes unresponsive).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `delay` - The voting delay in ledgers.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_voting_delay(e: &Env, delay: u32) {
    e.storage().instance().set(&GovernorStorageKey::VotingDelay, &delay);
}

/// Sets the voting period.
///
/// The period value is not validated here. It is the responsibility of
/// the implementer to ensure that the period is appropriate (e.g., enough
/// time for voters to participate, but not so long that urgent actions
/// cannot be taken).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `period` - The voting period in ledgers.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_voting_period(e: &Env, period: u32) {
    e.storage().instance().set(&GovernorStorageKey::VotingPeriod, &period);
}

/// Sets the address of the token contract that implements the Votes trait.
///
/// This function can only be called **once**. It is expected to be called
/// during the constructor of the governor contract. Subsequent calls will
/// fail with [`GovernorError::TokenContractAlreadySet`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_contract` - The address of the token contract.
///
/// # Errors
///
/// * [`GovernorError::TokenContractAlreadySet`] - Occurs if the token contract
///   has already been set.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_token_contract(e: &Env, token_contract: &Address) {
    let key = GovernorStorageKey::TokenContract;
    if e.storage().instance().has(&key) {
        panic_with_error!(e, GovernorError::TokenContractAlreadySet);
    }
    e.storage().instance().set(&key, token_contract);
}

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
/// * refer to [`get_proposal_threshold()`] errors.
/// * refer to [`get_voting_delay()`] errors.
/// * refer to [`get_voting_period()`] errors.
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
) -> BytesN<32> {
    // Validate proposal length
    let targets_len = targets.len();
    if targets_len == 0 {
        panic_with_error!(e, GovernorError::EmptyProposal);
    }
    if targets_len != functions.len() || targets_len != args.len() {
        panic_with_error!(e, GovernorError::InvalidProposalLength);
    }

    // Use previous ledger to prevent flash loan based proposals
    let snapshot = e.ledger().sequence() - 1;
    let proposer_votes = get_voting_power(e, proposer, snapshot as u64); // TODO: revert `as u64` cast to snapshot if we go with ledgers instead of
                                                                         // timestamps.

    // Check proposer has sufficient voting power
    let threshold = get_proposal_threshold(e);
    if proposer_votes < threshold {
        panic_with_error!(e, GovernorError::InsufficientProposerVotes);
    }

    let current_ledger = e.ledger().sequence();

    // Compute proposal ID
    let description_hash = e.crypto().keccak256(&description.clone().to_xdr(e)).to_bytes();
    let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);

    // Check proposal doesn't already exist
    if e.storage().persistent().has(&GovernorStorageKey::Proposal(proposal_id.clone())) {
        panic_with_error!(e, GovernorError::ProposalAlreadyExists);
    }

    // Calculate voting schedule
    let voting_delay = get_voting_delay(e);
    let voting_period = get_voting_period(e);
    let Some(vote_start) = current_ledger.checked_add(voting_delay) else {
        panic_with_error!(e, GovernorError::MathOverflow);
    };
    let Some(vote_end) = vote_start.checked_add(voting_period) else {
        panic_with_error!(e, GovernorError::MathOverflow);
    };

    // Store proposal
    let proposal = ProposalCore {
        proposer: proposer.clone(),
        vote_start,
        vote_end,
        state: ProposalState::Pending,
    };
    e.storage().persistent().set(&GovernorStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    emit_proposal_created(
        e,
        &proposal_id,
        proposer,
        &targets,
        &functions,
        &args,
        vote_start,
        vote_end,
        &description,
    );

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
/// * `executor` - The address executing the proposal.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotSuccessful`] - Occurs if the proposal has not
///   succeeded.
/// * [`GovernorError::ProposalAlreadyExecuted`] - Occurs if the proposal has
///   already been executed.
/// * refer to [`get_proposal_core()`] errors.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn execute(
    e: &Env,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description_hash: &BytesN<32>,
) -> BytesN<32> {
    let proposal_id = hash_proposal(e, &targets, &functions, &args, description_hash);

    // Get proposal and verify it exists
    let mut proposal = get_proposal_core(e, &proposal_id);

    // Check proposal state
    let state = derive_proposal_state(e, &proposal_id, &proposal);
    if state == ProposalState::Executed {
        panic_with_error!(e, GovernorError::ProposalAlreadyExecuted);
    }
    if state != ProposalState::Succeeded {
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
    e.storage().persistent().set(&GovernorStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    crate::governor::emit_proposal_executed(e, &proposal_id);

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
/// * `operator` - The address cancelling the proposal.
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

    // Blacklist non-cancellable states
    let state = derive_proposal_state(e, &proposal_id, &proposal);
    match state {
        ProposalState::Canceled | ProposalState::Expired | ProposalState::Executed => {
            panic_with_error!(e, GovernorError::ProposalNotCancellable)
        }
        _ => {}
    }

    // Mark as cancelled
    proposal.state = ProposalState::Canceled;
    e.storage().persistent().set(&GovernorStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    crate::governor::emit_proposal_cancelled(e, &proposal_id);

    proposal_id
}

// ################## HELPERS ##################

/// Computes and returns the proposal ID from the proposal parameters.
///
/// The proposal ID is a deterministic keccak256 hash of the targets, functions,
/// args, and description hash. This allows anyone to compute the ID
/// without storing the full proposal data.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description_hash` - The hash of the proposal description.
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

/// Prepares a vote by verifying the proposal is active,
/// and returning the proposal snapshot ledger for voting power lookup.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `voter` - The address casting the vote.
/// * `proposal_id` - The unique identifier of the proposal.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotActive`] - Occurs if the proposal is not in
///   the active state.
/// * refer to [`get_proposal_core()`] errors.
pub fn check_proposal_state(e: &Env, proposal_id: &BytesN<32>) -> u32 {
    let core = get_proposal_core(e, proposal_id);
    let state = derive_proposal_state(e, proposal_id, &core);
    if state != ProposalState::Active {
        panic_with_error!(e, GovernorError::ProposalNotActive);
    }

    core.vote_start
}

fn derive_proposal_state(e: &Env, proposal_id: &BytesN<32>, core: &ProposalCore) -> ProposalState {
    match core.state {
        ProposalState::Canceled | ProposalState::Succeeded | ProposalState::Executed => {
            return core.state;
        }
        _ => {}
    }

    let current_ledger = e.ledger().sequence();

    // `vote_start` is the snapshot ledger, so voting opens on the next ledger.
    if current_ledger <= core.vote_start {
        return ProposalState::Pending;
    }

    if current_ledger <= core.vote_end {
        return ProposalState::Active;
    }

    // Voting has ended — check whether the proposal met quorum and majority.
    if quorum_reached(e, proposal_id) && tally_succeeded(e, proposal_id) {
        return ProposalState::Succeeded;
    }

    ProposalState::Defeated
}

// ################## COUNTING: QUERY STATE ##################

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
    let key = GovernorStorageKey::HasVoted(proposal_id.clone(), account.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, GOVERNOR_TTL_THRESHOLD, GOVERNOR_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Returns the quorum value.
///
/// The quorum is the minimum total voting power (for + abstain) that must
/// participate for a proposal to be valid. A single quorum value is shared
/// across all proposal tallies.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::QuorumNotSet`] - Occurs if the quorum has not been set.
pub fn get_quorum(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::Quorum)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::QuorumNotSet))
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
/// * [`GovernorError::MathOverflow`] - Occurs if participation tally overflows.
/// * also refer to [`get_quorum()`] errors.
pub fn quorum_reached(e: &Env, proposal_id: &BytesN<32>) -> bool {
    let quorum = get_quorum(e);
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
    let key = GovernorStorageKey::ProposalVote(proposal_id.clone());
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

// ################## COUNTING: CHANGE STATE ##################

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
    let old_quorum = e.storage().instance().get(&GovernorStorageKey::Quorum).unwrap_or(0u128);
    e.storage().instance().set(&GovernorStorageKey::Quorum, &quorum);
    crate::governor::emit_quorum_changed(e, old_quorum, quorum);
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
    let voted_key = GovernorStorageKey::HasVoted(proposal_id.clone(), account.clone());
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
    let vote_key = GovernorStorageKey::ProposalVote(proposal_id.clone());
    e.storage().persistent().set(&vote_key, &counts);

    // Mark account as having voted
    e.storage().persistent().set(&voted_key, &true);
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
pub fn cast_vote(
    e: &Env,
    proposal_id: &BytesN<32>,
    vote_type: u32,
    reason: &String,
    voter: &Address,
) -> u128 {
    let snapshot = check_proposal_state(e, proposal_id);
    let voter_weight = get_voting_power(e, voter, snapshot as u64); // TODO: revert `as u64` cast to snapshot if we go with ledgers instead of
                                                                    // timestamps.
    count_vote(e, proposal_id, voter, vote_type, voter_weight);
    emit_vote_cast(e, voter, proposal_id, vote_type, voter_weight, reason);
    voter_weight
}

// ################## INTERNAL HELPERS ##################

/// Fetches the voting power of an account at a specific timepoint from the
/// token contract via a cross-contract call to `get_votes_at_checkpoint`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to query voting power for.
/// * `timepoint` - The timepoint to query.
///
/// # Errors
///
/// * refer to [`get_token_contract()`] errors.
fn get_voting_power(e: &Env, account: &Address, timepoint: u64) -> u128 {
    let token = get_token_contract(e);
    e.invoke_contract(
        &token,
        &Symbol::new(e, "get_votes_at_checkpoint"),
        soroban_sdk::vec![e, account.into_val(e), timepoint.into_val(e)],
    )
}
