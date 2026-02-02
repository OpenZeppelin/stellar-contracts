//! # Governor Module
//!
//! Implements on-chain governance functionality for Soroban contracts.
//!
//! This module provides the core governance primitives for decentralized
//! decision-making, including proposal creation, voting, and execution.
//!
//! ## Structure
//!
//! The base module includes:
//!
//! - Proposal lifecycle management (creation, voting, execution, cancellation)
//! - Integration with Votes interface for voting power snapshots
//!
//! The following optional extensions are available:
//!
//! - *GovernorSettings* provides configurable parameters like voting delay,
//!   voting period, and proposal threshold.
//! - *Quorum* defines quorum as a percentage of total voting power.
//! - *TimelockControl* integrates with the Timelock contract for
//!   delayed execution (queue step before execute).
//! - *Votes* integrates with a token contract that implements the
//!   Votes interface.
//! - *CountingSimple* provides standard For/Against/Abstain counting.
//!
//! ## Governance Flow
//!
//! 1. **Propose**: A user with sufficient voting power creates a proposal
//! 2. **Vote**: Token holders vote during the voting period
//! 3. **Execute**: Successful proposals can be executed
//!
//! When using the *TimelockControl* extension, an additional step is
//! added between voting and execution:
//!
//! 1. **Propose** → 2. **Vote** → 3. **Queue** → 4. **Execute**

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, contracttype, Address, BytesN, Env, String,
    Symbol, Val, Vec,
};
pub use storage::*;

// ################## TRAIT ##################

/// Base Governor Trait
///
/// The `Governor` trait defines the core functionality for on-chain governance.
/// It provides a standard interface for creating proposals, voting, and
/// executing approved actions.
#[contracttrait]
pub trait Governor {
    /// Returns the name of the governor.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn name(e: &Env) -> String {
        storage::get_name(e)
    }

    /// Returns the version of the governor contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn version(e: &Env) -> String {
        storage::get_version(e)
    }

    /// Returns the voting delay in ledgers.
    ///
    /// The voting delay is the number of ledgers between proposal creation
    /// and the start of voting.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn voting_delay(e: &Env) -> u32 {
        storage::get_voting_delay(e)
    }

    /// Returns the voting period in ledgers.
    ///
    /// The voting period is the number of ledgers during which voting is open.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn voting_period(e: &Env) -> u32 {
        storage::get_voting_period(e)
    }

    /// Returns the minimum voting power required to create a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn proposal_threshold(e: &Env) -> u128 {
        storage::get_proposal_threshold(e)
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
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    fn proposal_state(e: &Env, proposal_id: BytesN<32>) -> ProposalState {
        storage::get_proposal_state(e, &proposal_id)
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
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
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
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
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
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    fn proposal_proposer(e: &Env, proposal_id: BytesN<32>) -> Address {
        storage::get_proposal_proposer(e, &proposal_id)
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
    /// # Returns
    ///
    /// The unique identifier (hash) of the created proposal.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::InsufficientProposerVotes`] - If the proposer does
    ///   not have enough voting power.
    /// * [`GovernorError::ProposalAlreadyExists`] - If a proposal with the
    ///   same parameters already exists.
    /// * [`GovernorError::InvalidProposalLength`] - If the targets, functions,
    ///   and args vectors have different lengths.
    /// * [`GovernorError::EmptyProposal`] - If the proposal contains no
    ///   actions.
    ///
    /// # Events
    ///
    /// * topics - `["proposal_created", proposal_id: BytesN<32>, proposer:
    ///   Address]`
    /// * data - `[targets: Vec<Address>, functions: Vec<Symbol>, args:
    ///   Vec<Vec<Val>>, vote_start: u32, vote_end: u32, description: String]`
    fn propose(
        e: &Env,
        proposer: Address,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description: String,
    ) -> BytesN<32> {
        storage::propose(e, &proposer, targets, functions, args, description)
    }

    /// Casts a vote on a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `voter` - The address casting the vote.
    /// * `proposal_id` - The unique identifier of the proposal.
    /// * `vote_type` - The type of vote. The interpretation depends on the
    ///   counting module used. For simple counting: 0 = Against, 1 = For,
    ///   2 = Abstain.
    /// * `reason` - An optional explanation for the vote.
    ///
    /// # Returns
    ///
    /// The voting power of the voter.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::ProposalNotActive`] - If voting is not currently
    ///   open.
    /// * [`GovernorError::AlreadyVoted`] - If the voter has already voted.
    /// * [`GovernorError::InvalidVoteType`] - If the vote type is not valid
    ///   for the counting module.
    ///
    /// # Events
    ///
    /// * topics - `["vote_cast", voter: Address, proposal_id: BytesN<32>]`
    /// * data - `[vote_type: u8, weight: u128, reason: String]`
    fn cast_vote(
        e: &Env,
        voter: Address,
        proposal_id: BytesN<32>,
        vote_type: u8,
        reason: String,
    ) -> u128 {
        storage::cast_vote(e, &voter, &proposal_id, vote_type, reason)
    }

    /// Executes a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `targets` - The addresses of contracts to call.
    /// * `functions` - The function names to invoke on each target.
    /// * `args` - The arguments for each function call.
    /// * `description_hash` - The hash of the proposal description.
    ///
    /// # Returns
    ///
    /// The unique identifier of the proposal.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::ProposalNotQueued`] - If the proposal has not been
    ///   queued (only relevant when using queuing extensions).
    ///
    /// # Events
    ///
    /// * topics - `["proposal_executed", proposal_id: BytesN<32>]`
    /// * data - `[]`
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
    ) -> BytesN<32> {
        storage::execute(e, targets, functions, args, &description_hash)
    }

    /// Cancels a proposal.
    ///
    /// Can only be called by the proposer before the proposal is executed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `targets` - The addresses of contracts to call.
    /// * `functions` - The function names to invoke on each target.
    /// * `args` - The arguments for each function call.
    /// * `description_hash` - The hash of the proposal description.
    ///
    /// # Returns
    ///
    /// The unique identifier of the cancelled proposal.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::ProposalAlreadyExecuted`] - If the proposal has
    ///   already been executed.
    ///
    /// # Events
    ///
    /// * topics - `["proposal_cancelled", proposal_id: BytesN<32>]`
    /// * data - `[]`
    fn cancel(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
    ) -> BytesN<32> {
        storage::cancel(e, targets, functions, args, &description_hash)
    }

    /// Returns the voting power of an account at a specific ledger.
    ///
    /// This function queries the configured Votes contract to retrieve
    /// the historical voting power of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query voting power for.
    /// * `ledger` - The ledger number at which to evaluate voting power.
    fn get_votes(e: &Env, account: Address, ledger: u32) -> u128 {
        storage::get_votes(e, &account, ledger)
    }

    /// Returns a string describing how votes are counted.
    ///
    /// This is used by UIs to display the counting mode to users. The format
    /// follows a URI-like pattern, e.g., `"support=bravo&quorum=for,abstain"`. // TODO: change the wording here, it is cryptic
    ///
    /// The base implementation returns an empty string. Counting extensions
    /// (like *CountingSimple*) should override this to return their mode.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn counting_mode(e: &Env) -> String {
        String::from_str(e, "")
    }

    /// Returns whether proposals need to be queued before execution.
    ///
    /// The base implementation returns `false`, meaning proposals can be
    /// executed directly after success. Extensions like *TimelockControl*
    /// should override this to return `true`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn proposal_needs_queuing(_e: &Env) -> bool {
        false
    }
}

// ################## TYPES ##################

/// The state of a proposal in its lifecycle.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ProposalState {
    /// The proposal is pending and voting has not started yet.
    Pending = 0,
    /// The proposal is active and voting is ongoing.
    Active = 1,
    /// The proposal has been cancelled.
    Canceled = 2,
    /// The proposal was defeated (did not meet quorum or majority).
    Defeated = 3,
    /// The proposal succeeded and can be executed. If there is a queuing extension enabled, this state means the proposal is ready to be queued.
    Succeeded = 4,
    /// The proposal is queued for execution (when using an extension that enables the queue step, like TimelockControl).
    Queued = 5,
    /// The proposal has expired and can no longer be executed.
    Expired = 6,
    /// The proposal has been executed.
    Executed = 7,
}

// ################## ERRORS ##################

/// Errors that can occur in governor operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GovernorError {
    /// The proposal was not found.
    ProposalNotFound = 5000,
    /// The proposal already exists.
    ProposalAlreadyExists = 5001,
    /// The proposer does not have enough voting power.
    InsufficientProposerVotes = 5002,
    /// The proposal contains no actions.
    EmptyProposal = 5003,
    /// The targets, functions, and args vectors have different lengths.
    InvalidProposalLength = 5004,
    /// The proposal is not in the active state.
    ProposalNotActive = 5005,
    /// The voter has already voted on this proposal.
    AlreadyVoted = 5006,
    /// The proposal has not succeeded.
    ProposalNotSuccessful = 5007,
    /// The proposal has not been queued.
    ProposalNotQueued = 5008,
    /// The proposal has already been executed.
    ProposalAlreadyExecuted = 5009,
    /// The caller is not authorized to perform this action.
    Unauthorized = 5010,
    /// The voting delay has not been set.
    VotingDelayNotSet = 5011,
    /// The voting period has not been set.
    VotingPeriodNotSet = 5012,
    /// The proposal threshold has not been set.
    ProposalThresholdNotSet = 5013,
    /// The votes contract address has not been set.
    VotesContractNotSet = 5014,
    /// Invalid voting delay value.
    InvalidVotingDelay = 5016,
    /// Invalid voting period value.
    InvalidVotingPeriod = 5017,
    /// Invalid proposal threshold value.
    InvalidProposalThreshold = 5018,
    /// The name has not been set.
    NameNotSet = 5019,
    /// The version has not been set.
    VersionNotSet = 5020,
    /// The vote type is not valid for the counting module.
    InvalidVoteType = 5021,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL threshold for extending storage entries (in ledgers)
pub const GOVERNOR_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL extension amount for storage entries (in ledgers)
pub const GOVERNOR_TTL_THRESHOLD: u32 = GOVERNOR_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when a proposal is created.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCreated {
    #[topic]
    pub proposal_id: BytesN<32>,
    #[topic]
    pub proposer: Address,
    pub targets: Vec<Address>,
    pub functions: Vec<Symbol>,
    pub args: Vec<Vec<Val>>,
    pub vote_start: u32,
    pub vote_end: u32,
    pub description: String,
}

/// Emits an event when a proposal is created.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `proposer` - The address that created the proposal.
/// * `targets` - The addresses of contracts to call.
/// * `functions` - The function names to invoke on each target.
/// * `args` - The arguments for each function call.
/// * `vote_start` - The ledger number when voting starts.
/// * `vote_end` - The ledger number when voting ends.
/// * `description` - The proposal description.
#[allow(clippy::too_many_arguments)]
pub fn emit_proposal_created(
    e: &Env,
    proposal_id: &BytesN<32>,
    proposer: &Address,
    targets: &Vec<Address>,
    functions: &Vec<Symbol>,
    args: &Vec<Vec<Val>>,
    vote_start: u32,
    vote_end: u32,
    description: &String,
) {
    ProposalCreated {
        proposal_id: proposal_id.clone(),
        proposer: proposer.clone(),
        targets: targets.clone(),
        functions: functions.clone(),
        args: args.clone(),
        vote_start,
        vote_end,
        description: description.clone(),
    }
    .publish(e);
}

/// Event emitted when a vote is cast.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCast {
    #[topic]
    pub voter: Address,
    #[topic]
    pub proposal_id: BytesN<32>,
    /// The type of vote cast.
    pub vote_type: u8,
    /// The voting power used.
    pub weight: u128,
    /// The voter's explanation for their vote.
    pub reason: String,
}

/// Emits an event when a vote is cast.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `voter` - The address that cast the vote.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `vote_type` - The type of vote cast.
/// * `weight` - The voting power of the voter.
/// * `reason` - The voter's explanation for their vote.
pub fn emit_vote_cast(
    e: &Env,
    voter: &Address,
    proposal_id: &BytesN<32>,
    vote_type: u8,
    weight: u128,
    reason: &String,
) {
    VoteCast {
        voter: voter.clone(),
        proposal_id: proposal_id.clone(),
        vote_type,
        weight,
        reason: reason.clone(),
    }
    .publish(e);
}

/// Event emitted when a proposal is queued.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalQueued {
    #[topic]
    pub proposal_id: BytesN<32>,
    pub eta: u64,
}

/// Emits an event when a proposal is queued.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `eta` - The estimated time of execution (timestamp).
pub fn emit_proposal_queued(e: &Env, proposal_id: &BytesN<32>, eta: u64) {
    ProposalQueued { proposal_id: proposal_id.clone(), eta }.publish(e);
}

/// Event emitted when a proposal is executed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalExecuted {
    #[topic]
    pub proposal_id: BytesN<32>,
}

/// Emits an event when a proposal is executed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
pub fn emit_proposal_executed(e: &Env, proposal_id: &BytesN<32>) {
    ProposalExecuted { proposal_id: proposal_id.clone() }.publish(e);
}

/// Event emitted when a proposal is cancelled.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCancelled {
    #[topic]
    pub proposal_id: BytesN<32>,
}

/// Emits an event when a proposal is cancelled.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
pub fn emit_proposal_cancelled(e: &Env, proposal_id: &BytesN<32>) {
    ProposalCancelled { proposal_id: proposal_id.clone() }.publish(e);
}
