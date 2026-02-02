//! # Governor Storage Module
//!
//! This module provides storage utilities for the Governor contract.
//! It defines storage keys and helper functions for managing proposal state,
//! votes, and configuration parameters.

use soroban_sdk::{contracttype, Address, BytesN, Env, String, Symbol, Val, Vec};

use crate::governor::{GovernorError, ProposalState};

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
    /// The proposal threshold.
    ProposalThreshold,
    /// The address of the Votes contract.
    VotesContract,
    /// Proposal data indexed by proposal ID.
    Proposal(BytesN<32>),
    /// Vote receipt for a specific voter and proposal.
    VoteReceipt(BytesN<32>, Address),
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
    /// Whether the proposal has been executed.
    pub executed: bool,
    /// Whether the proposal has been cancelled.
    pub cancelled: bool,
}

/// Vote tallies for a proposal.
#[derive(Clone)]
#[contracttype]
pub struct ProposalVotes {
    /// Votes in favor.
    pub for_votes: u128,
    /// Votes against.
    pub against_votes: u128,
    /// Abstain votes.
    pub abstain_votes: u128,
}

/// Receipt of a vote cast by a voter.
#[derive(Clone)]
#[contracttype]
pub struct VoteReceipt {
    /// Whether the voter has voted.
    pub has_voted: bool,
    /// The type of vote cast.
    pub vote_type: u8,
    /// The voting power used.
    pub votes: u128,
}

// ################## GETTER FUNCTIONS ##################

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

/// Returns the address of the Votes contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`GovernorError::VotesContractNotSet`] - Occurs if the votes contract has
///   not been set.
pub fn get_votes_contract(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::VotesContract)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotesContractNotSet))
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
    e.storage()
        .persistent()
        .get(&GovernorStorageKey::Proposal(proposal_id.clone()))
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::ProposalNotFound))
}

/// Returns the current state of a proposal.
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
pub fn get_proposal_state(e: &Env, proposal_id: &BytesN<32>) -> ProposalState {
    let core = get_proposal_core(e, proposal_id);
    let current_ledger = e.ledger().sequence();

    if core.executed {
        return ProposalState::Executed;
    }

    if core.cancelled {
        return ProposalState::Canceled;
    }

    if current_ledger < core.vote_start {
        return ProposalState::Pending;
    }

    if current_ledger <= core.vote_end {
        return ProposalState::Active;
    }

    // Voting has ended - determine if succeeded or defeated
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

/// Returns whether an account has voted on a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `account` - The address to check.
pub fn has_voted(e: &Env, proposal_id: &BytesN<32>, account: &Address) -> bool {
    let key = GovernorStorageKey::VoteReceipt(proposal_id.clone(), account.clone());
    e.storage()
        .persistent()
        .get::<_, VoteReceipt>(&key)
        .map(|receipt| receipt.has_voted)
        .unwrap_or(false)
}

// ################## SETTER FUNCTIONS ##################

// TODO: for the setters below, set only once logic should be added

/// Sets the name of the governor.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The name to set.
pub fn set_name(e: &Env, name: String) {
    e.storage().instance().set(&GovernorStorageKey::Name, &name);
}

/// Sets the version of the governor contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `version` - The version string to set.
pub fn set_version(e: &Env, version: String) {
    e.storage().instance().set(&GovernorStorageKey::Version, &version);
}

/// Sets the proposal threshold.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `threshold` - The minimum voting power required to create a proposal.
pub fn set_proposal_threshold(e: &Env, threshold: u128) {
    e.storage().instance().set(&GovernorStorageKey::ProposalThreshold, &threshold);
}

/// Sets the voting delay.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `delay` - The voting delay in ledgers.
pub fn set_voting_delay(e: &Env, delay: u32) {
    e.storage().instance().set(&GovernorStorageKey::VotingDelay, &delay);
}

/// Sets the voting period.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `period` - The voting period in ledgers.
pub fn set_voting_period(e: &Env, period: u32) {
    e.storage().instance().set(&GovernorStorageKey::VotingPeriod, &period);
}

/// Sets the address of the Votes contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `votes_contract` - The address of the Votes contract.
pub fn set_votes_contract(e: &Env, votes_contract: &Address) {
    e.storage().instance().set(&GovernorStorageKey::VotesContract, votes_contract);
}

// ################## PROPOSAL FUNCTIONS ##################

/// Creates a new proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `proposer` - The address creating the proposal.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `description` - A description of the proposal.
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
pub fn propose(
    e: &Env,
    proposer: &Address,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description: String,
) -> BytesN<32> {
    // Require authorization from the proposer
    proposer.require_auth();

    // Validate proposal length
    let targets_len = targets.len();
    if targets_len == 0 {
        panic_with_error!(e, GovernorError::EmptyProposal);
    }
    if targets_len != functions.len() || targets_len != args.len() {
        panic_with_error!(e, GovernorError::InvalidProposalLength);
    }

    // Check proposer has sufficient voting power
    let current_ledger = e.ledger().sequence();
    let proposer_votes = get_votes(e, proposer, current_ledger);
    let threshold = get_proposal_threshold(e);
    if proposer_votes < threshold {
        panic_with_error!(e, GovernorError::InsufficientProposerVotes);
    }

    // Compute proposal ID
    let description_hash = e.crypto().keccak256(&description.to_xdr(e)).to_bytes();
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
        executed: false,
        cancelled: false,
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

/// Casts a vote on a proposal.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `voter` - The address casting the vote.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `vote_type` - The type of vote (interpretation depends on counting module).
/// * `reason` - An optional explanation for the vote.
///
/// # Errors
///
/// * [`GovernorError::ProposalNotActive`] - Occurs if the proposal is not in
///   the active state.
/// * [`GovernorError::AlreadyVoted`] - Occurs if the voter has already voted
///   on this proposal.
pub fn cast_vote(
    e: &Env,
    voter: &Address,
    proposal_id: &BytesN<32>,
    vote_type: u8,
    reason: String,
) -> u128 {
    // Require authorization from the voter
    voter.require_auth();

    // Check proposal is active
    let state = get_proposal_state(e, proposal_id);
    if state != ProposalState::Active {
        panic_with_error!(e, GovernorError::ProposalNotActive);
    }

    // Check voter hasn't already voted
    let receipt_key = GovernorStorageKey::VoteReceipt(proposal_id.clone(), voter.clone());
    if e.storage().persistent().has(&receipt_key) {
        let existing: VoteReceipt = e.storage().persistent().get(&receipt_key).unwrap();
        if existing.has_voted {
            panic_with_error!(e, GovernorError::AlreadyVoted);
        }
    }

    // Get voter's voting power at the proposal snapshot
    let snapshot = get_proposal_snapshot(e, proposal_id);
    let weight = get_votes(e, voter, snapshot);

    // Record the vote
    let receipt = VoteReceipt { has_voted: true, vote_type, votes: weight };
    e.storage().persistent().set(&receipt_key, &receipt);

    // Emit event
    crate::governor::emit_vote_cast(e, voter, proposal_id, vote_type, weight, &reason);

    weight
}

/// Executes a successful proposal.
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
/// * [`GovernorError::ProposalNotSuccessful`] - Occurs if the proposal has not
///   succeeded.
/// * [`GovernorError::ProposalAlreadyExecuted`] - Occurs if the proposal has
///   already been executed.
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

    // Mark as executed
    let mut proposal = get_proposal_core(e, &proposal_id);
    proposal.executed = true;
    e.storage().persistent().set(&GovernorStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Execute each action
    for i in 0..targets.len() {
        let target = targets.get(i).unwrap();
        let function = functions.get(i).unwrap();
        let func_args = args.get(i).unwrap();
        e.invoke_contract::<Val>(&target, &function, func_args);
    }

    // Emit event
    crate::governor::emit_proposal_executed(e, &proposal_id);

    proposal_id
}

/// Cancels a proposal.
///
/// Can only be called by the proposer before execution.
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
/// * [`GovernorError::ProposalNotFound`] - Occurs if the proposal does not
///   exist.
/// * [`GovernorError::ProposalAlreadyExecuted`] - Occurs if the proposal has
///   already been executed.
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

    // Only proposer can cancel
    proposal.proposer.require_auth();

    // Cannot cancel if already executed
    if proposal.executed {
        panic_with_error!(e, GovernorError::ProposalAlreadyExecuted);
    }

    // Mark as cancelled
    proposal.cancelled = true;
    e.storage().persistent().set(&GovernorStorageKey::Proposal(proposal_id.clone()), &proposal);

    // Emit event
    crate::governor::emit_proposal_cancelled(e, &proposal_id);

    proposal_id
}

/// Returns the voting power of an account at a specific ledger.
///
/// This queries the configured Votes contract using the `get_past_votes` function.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to query voting power for.
/// * `ledger` - The ledger number at which to evaluate voting power.
pub fn get_votes(e: &Env, account: &Address, ledger: u32) -> u128 {
    let votes_contract = get_votes_contract(e);
    e.invoke_contract(
        &votes_contract,
        &Symbol::new(e, "get_past_votes"),
        soroban_sdk::vec![e, account.to_val(), ledger.into()],
    )
}

// ################## UTILITY FUNCTIONS ##################

/// Computes the proposal ID from the proposal parameters.
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
