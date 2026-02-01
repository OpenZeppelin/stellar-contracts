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
    /// The address of the Timelock contract.
    TimelockContract,
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
/// # Panics
///
/// Panics with [`GovernorError::NameNotSet`] if the name has not been set.
pub fn get_name(e: &Env) -> String {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::Name)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::NameNotSet))
}

/// Returns the version of the governor contract.
///
/// # Panics
///
/// Panics with [`GovernorError::VersionNotSet`] if the version has not been set.
pub fn get_version(e: &Env) -> String {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::Version)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VersionNotSet))
}

/// Returns the voting delay in ledgers.
///
/// # Panics
///
/// Panics with [`GovernorError::VotingDelayNotSet`] if the voting delay has not been set.
pub fn get_voting_delay(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::VotingDelay)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotingDelayNotSet))
}

/// Returns the voting period in ledgers.
///
/// # Panics
///
/// Panics with [`GovernorError::VotingPeriodNotSet`] if the voting period has not been set.
pub fn get_voting_period(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::VotingPeriod)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotingPeriodNotSet))
}

/// Returns the proposal threshold.
///
/// # Panics
///
/// Panics with [`GovernorError::ProposalThresholdNotSet`] if the proposal threshold has not been set.
pub fn get_proposal_threshold(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::ProposalThreshold)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::ProposalThresholdNotSet))
}

/// Returns the address of the Votes contract.
///
/// # Panics
///
/// Panics with [`GovernorError::VotesContractNotSet`] if the votes contract has not been set.
pub fn get_votes_contract(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::VotesContract)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotesContractNotSet))
}

/// Returns the address of the Timelock contract.
///
/// # Panics
///
/// Panics with [`GovernorError::TimelockContractNotSet`] if the timelock contract has not been set.
pub fn get_timelock_contract(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&GovernorStorageKey::TimelockContract)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::TimelockContractNotSet))
}

/// Returns the quorum for a given ledger.
///
/// This is a placeholder implementation. Extensions like QuorumFraction
/// will override this behavior.
pub fn get_quorum(_e: &Env, _ledger: u32) -> u128 {
    // Default implementation returns 0
    // Extensions should override this
    0
}

/// Returns the core proposal data.
///
/// # Panics
///
/// Panics with [`GovernorError::ProposalNotFound`] if the proposal does not exist.
pub fn get_proposal_core(e: &Env, proposal_id: &BytesN<32>) -> ProposalCore {
    e.storage()
        .persistent()
        .get(&GovernorStorageKey::Proposal(proposal_id.clone()))
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::ProposalNotFound))
}

/// Returns the current state of a proposal.
///
/// # Panics
///
/// Panics with [`GovernorError::ProposalNotFound`] if the proposal does not exist.
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
    // This is a simplified implementation
    // Extensions may override this logic
    ProposalState::Defeated
}

/// Returns the snapshot ledger for a proposal.
///
/// # Panics
///
/// Panics with [`GovernorError::ProposalNotFound`] if the proposal does not exist.
pub fn get_proposal_snapshot(e: &Env, proposal_id: &BytesN<32>) -> u32 {
    let core = get_proposal_core(e, proposal_id);
    core.vote_start
}

/// Returns the deadline ledger for a proposal.
///
/// # Panics
///
/// Panics with [`GovernorError::ProposalNotFound`] if the proposal does not exist.
pub fn get_proposal_deadline(e: &Env, proposal_id: &BytesN<32>) -> u32 {
    let core = get_proposal_core(e, proposal_id);
    core.vote_end
}

/// Returns the proposer of a proposal.
///
/// # Panics
///
/// Panics with [`GovernorError::ProposalNotFound`] if the proposal does not exist.
pub fn get_proposal_proposer(e: &Env, proposal_id: &BytesN<32>) -> Address {
    let core = get_proposal_core(e, proposal_id);
    core.proposer
}

/// Returns whether an account has voted on a proposal.
pub fn has_voted(e: &Env, proposal_id: &BytesN<32>, account: &Address) -> bool {
    let key = GovernorStorageKey::VoteReceipt(proposal_id.clone(), account.clone());
    e.storage()
        .persistent()
        .get::<_, VoteReceipt>(&key)
        .map(|receipt| receipt.has_voted)
        .unwrap_or(false)
}

// ################## SETTER FUNCTIONS ##################

/// Sets the name of the governor.
pub fn set_name(e: &Env, name: String) {
    e.storage().instance().set(&GovernorStorageKey::Name, &name);
}

/// Sets the version of the governor contract.
pub fn set_version(e: &Env, version: String) {
    e.storage().instance().set(&GovernorStorageKey::Version, &version);
}

/// Sets the voting delay.
pub fn set_voting_delay(e: &Env, delay: u32) {
    e.storage().instance().set(&GovernorStorageKey::VotingDelay, &delay);
}

/// Sets the voting period.
pub fn set_voting_period(e: &Env, period: u32) {
    e.storage().instance().set(&GovernorStorageKey::VotingPeriod, &period);
}

/// Sets the proposal threshold.
pub fn set_proposal_threshold(e: &Env, threshold: u128) {
    e.storage().instance().set(&GovernorStorageKey::ProposalThreshold, &threshold);
}

/// Sets the votes contract address.
pub fn set_votes_contract(e: &Env, contract: Address) {
    e.storage().instance().set(&GovernorStorageKey::VotesContract, &contract);
}

/// Sets the timelock contract address.
pub fn set_timelock_contract(e: &Env, contract: Address) {
    e.storage().instance().set(&GovernorStorageKey::TimelockContract, &contract);
}

// ################## PROPOSAL FUNCTIONS ##################

/// Creates a new proposal.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn propose(
    _e: &Env,
    _proposer: &Address,
    _targets: Vec<Address>,
    _functions: Vec<Symbol>,
    _args: Vec<Vec<Val>>,
    _description: String,
) -> BytesN<32> {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}

/// Casts a vote on a proposal.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn cast_vote(
    _e: &Env,
    _voter: &Address,
    _proposal_id: &BytesN<32>,
    _vote_type: u8,
    _reason: String,
) -> u128 {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}

/// Queues a proposal for execution.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn queue(
    _e: &Env,
    _targets: Vec<Address>,
    _functions: Vec<Symbol>,
    _args: Vec<Vec<Val>>,
    _description_hash: &BytesN<32>,
) -> BytesN<32> {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}

/// Executes a queued proposal.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn execute(
    _e: &Env,
    _targets: Vec<Address>,
    _functions: Vec<Symbol>,
    _args: Vec<Vec<Val>>,
    _description_hash: &BytesN<32>,
) -> BytesN<32> {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}

/// Cancels a proposal.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn cancel(
    _e: &Env,
    _targets: Vec<Address>,
    _functions: Vec<Symbol>,
    _args: Vec<Vec<Val>>,
    _description_hash: &BytesN<32>,
) -> BytesN<32> {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}

/// Computes the proposal ID from the proposal parameters.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn hash_proposal(
    _e: &Env,
    _targets: &Vec<Address>,
    _functions: &Vec<Symbol>,
    _args: &Vec<Vec<Val>>,
    _description_hash: &BytesN<32>,
) -> BytesN<32> {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}

/// Returns the voting power of an account at a specific ledger.
///
/// This queries the configured Votes contract.
///
/// This is a placeholder implementation that will be fully implemented later.
pub fn get_votes(_e: &Env, _account: &Address, _ledger: u32) -> u128 {
    // Placeholder - will be implemented in the next phase
    panic!("Not implemented yet")
}
