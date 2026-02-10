//! # Governor Storage Module
//!
//! This module provides storage utilities for the Governor contract.
//! It defines storage keys and helper functions for managing proposal state,
//! votes, and configuration parameters.

use soroban_sdk::{
    contracttype, panic_with_error, xdr::ToXdr, Address, BytesN, Env, String, Symbol, Val, Vec,
};

use crate::governor::{
    GovernorError, ProposalState, GOVERNOR_EXTEND_AMOUNT, GOVERNOR_TTL_THRESHOLD,
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

    // These state transitions are ledger-dependent:
    //   Pending -> Active -> Defeated
    //   Queued  -> Expired
    //
    // Canceled, Succeeded, and Executed are ledger-independent — return
    // them immediately.
    match core.state {
        ProposalState::Canceled | ProposalState::Succeeded | ProposalState::Executed =>
            return core.state,
        _ => {}
    }

    // Derive Pending, Active, or Defeated from the current ledger.
    let current_ledger = e.ledger().sequence();

    if current_ledger < core.vote_start {
        return ProposalState::Pending;
    }

    if current_ledger <= core.vote_end {
        return ProposalState::Active;
    }

    // Voting has ended without the Counting module marking it as Succeeded.
    ProposalState::Defeated
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

/// Creates a new proposal and returns its unique identifier (proposal ID).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposer_votes` - The voting power of the proposer at the current ledger.
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
/// * refer to [`get_proposal_threshold()`] errors.
/// * refer to [`get_voting_delay()`] errors.
/// * refer to [`get_voting_period()`] errors.
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn propose(
    e: &Env,
    proposer_votes: u128,
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
    let vote_start = current_ledger + voting_delay;
    let vote_end = vote_start + voting_period;

    // Store proposal
    let proposal = ProposalCore {
        proposer: proposer.clone(),
        vote_start,
        vote_end,
        state: ProposalState::Pending,
    };
    e.storage().persistent().set(&GovernorStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    crate::governor::emit_proposal_created(
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
/// * refer to [`get_proposal_state()`] errors.
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

    // Check proposal state
    let state = get_proposal_state(e, &proposal_id);
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
    let mut proposal = get_proposal_core(e, &proposal_id);
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
/// * refer to [`get_proposal_state()`] errors.
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
    let state = get_proposal_state(e, &proposal_id);
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
    use soroban_sdk::Bytes;

    // Concatenate all inputs for hashing
    let mut data = Bytes::new(e);
    data.append(&targets.to_xdr(e));
    data.append(&functions.to_xdr(e));
    data.append(&args.to_xdr(e));
    data.append(&Bytes::from_slice(e, description_hash.to_array().as_slice()));

    e.crypto().keccak256(&data).to_bytes()
}

/// Prepares a vote by authorizing the voter, verifying the proposal is active,
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
/// * refer to [`get_proposal_state()`] errors.
pub fn prepare_vote(e: &Env, proposal_id: &BytesN<32>) -> u32 {
    let state = get_proposal_state(e, proposal_id);
    if state != ProposalState::Active {
        panic_with_error!(e, GovernorError::ProposalNotActive);
    }

    get_proposal_snapshot(e, proposal_id)
}
