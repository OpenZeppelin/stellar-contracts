//! # Governor Module
//!
//! Implements on-chain governance functionality for Soroban contracts.
//!
//! This module provides the core governance primitives for decentralized
//! decision-making, including proposal creation, voting, and execution.
//!
//! ## Structure
//!
//! The [`Governor`] trait includes:
//!
//! - Proposal lifecycle management (creation, voting, execution, cancellation)
//! - Integration with Votes interface for voting power snapshots
//!
//! The [`Governor`] trait needs the following concepts to be defined and made
//! available by the implementer:
//!
//! - *Votes*: Governor trait does not define how to store, manage, and access
//!   votes. But Governor trait needs to be able to access the voting power of
//!   an account at a specific ledger. A separate Votes trait is provided by
//!   this library as a default implementation. // TODO: hyperlink the Votes
//!   trait here, and revise the paragraph accordingly.
//!
//! - *Counting*: Governor trait does not define how votes are counted or how
//!   quorum is determined. A separate Counting trait defines the counting mode,
//!   quorum logic, and vote recording. // TODO: hyperlink the Counting trait
//!   here, and revise the paragraph accordingly.
//!
//! The following optional extensions are available:
//!
//! - *GovernorSettings* provides configurable parameters like voting delay,
//!   voting period, and proposal threshold.
//! - *TimelockControl* enables the optional `Queue` step in execution. It
//!   integrates the Governor Contract with the Timelock Contract for delayed
//!   execution (queue step before execute).
//!
//! ## Governance Flow
//!
//! 1. **Propose**: A user with sufficient voting power creates a proposal
//! 2. **Vote**: Token holders vote during the voting period
//! 3. **Execute**: Successful proposals (marked as `Succeeded` by the
//!    [`Counting`] module) can be executed
//!
//! When using an extension for `Queue` mechanism, like `TimelockControl`, an
//! additional `Queue` step is added between voting and execution:
//!
//! 1. **Propose** → 2. **Vote** → 3. **Queue** → 4. **Execute**
//!
//! # Security Considerations
//!
//! ## Flash Loan Voting Attack
//!
//! ### Vulnerability Overview
//!
//! Governance systems are vulnerable to flash loan attacks where an attacker
//! borrows a large amount of voting tokens, votes on a proposal, and returns
//! the tokens within the same transaction.
//!
//! ### Mitigation
//!
//! This implementation uses **snapshot-based voting power**. When a proposal
//! is created, the current ledger number is recorded as the "snapshot". All
//! voting power calculations use [`get_past_votes()`] which queries the
//! voting power at the snapshot ledger, not the current ledger.
//!
//! This means an attacker must hold tokens *before* a proposal is created
//! to have voting power on that proposal, making flash loan attacks
//! ineffective.
//!
//! ## Proposal Spam Attack
//!
//! ### Vulnerability Overview
//!
//! An attacker could create many proposals to overwhelm governance
//! participants, making it difficult to focus on legitimate proposals.
//!
//! ### Mitigation
//!
//! The **proposal threshold** ([`get_proposal_threshold()`]) requires
//! proposers to hold a minimum amount of voting power to create proposals.
//! This makes spam attacks economically costly.
//!
//! ## Governance Capture
//!
//! ### Vulnerability Overview
//!
//! An attacker could accumulate voting power over time to eventually control
//! governance decisions.
//!
//! ### Mitigation
//!
//! - **Quorum requirements** (defined by the [`Counting`] trait) ensure a
//!   minimum percentage of total voting supply participates in each proposal
//! - **Voting delay** ([`get_voting_delay()`]) gives token holders time to
//!   acquire more tokens or delegate before voting starts

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, contracttype, Address, BytesN, Env, String,
    Symbol, Val, Vec,
};

/// TODO: delete this after Votes PR is merged
pub trait Votes {
    fn get_past_votes(e: &Env, account: Address, ledger: u32) -> u128;
    fn get_past_total_supply(e: &Env, ledger: u32) -> u128;
}

/// TODO: delete this after Counting PR is merged
pub trait Counting {
    fn counting_mode(e: &Env) -> Symbol;
    fn has_voted(e: &Env, proposal_id: BytesN<32>, account: Address) -> bool;
    fn quorum(e: &Env, ledger: u32) -> u128;
    fn quorum_reached(e: &Env, proposal_id: BytesN<32>) -> bool;
    fn vote_succeeded(e: &Env, proposal_id: BytesN<32>) -> bool;
    fn count_vote(e: &Env, proposal_id: BytesN<32>, account: Address, vote_type: u32, weight: u128);
}

/// Base Governor Trait
///
/// The `Governor` trait defines the core functionality for on-chain governance.
/// It provides a standard interface for creating proposals, voting, and
/// executing approved actions.
///
/// The contract that implements this trait is expected to implement [`Votes`]
/// and [`Counting`] traits.
#[contracttrait]
pub trait Governor: Votes + Counting {
    /// Returns the name of the governor.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::NameNotSet`] - Occurs if the name has not been set.
    fn name(e: &Env) -> String {
        storage::get_name(e)
    }

    /// Returns the version of the governor contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::VersionNotSet`] - Occurs if the version has not been
    ///   set.
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
    ///
    /// # Errors
    ///
    /// * [`GovernorError::VotingDelayNotSet`] - Occurs if the voting delay has
    ///   not been set.
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
    ///
    /// # Errors
    ///
    /// * [`GovernorError::VotingPeriodNotSet`] - Occurs if the voting period
    ///   has not been set.
    fn voting_period(e: &Env) -> u32 {
        storage::get_voting_period(e)
    }

    /// Returns the minimum voting power required to create a proposal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::ProposalThresholdNotSet`] - Occurs if the proposal
    ///   threshold has not been set.
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

    /// Returns the proposal ID computed from the proposal details.
    ///
    /// The proposal ID is a deterministic keccak256 hash of the targets,
    /// functions, args, and description hash. This allows anyone to compute
    /// the ID without storing the full proposal data.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `targets` - The addresses of contracts to call.
    /// * `functions` - The function names to invoke on each target.
    /// * `args` - The arguments for each function call.
    /// * `description_hash` - The hash of the proposal description.
    fn get_proposal_id(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
    ) -> BytesN<32> {
        storage::hash_proposal(e, &targets, &functions, &args, &description_hash)
    }

    /// Creates a new proposal and returns its unique identifier (hash).
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
    /// * [`GovernorError::InsufficientProposerVotes`] - If the proposer does
    ///   not have enough voting power.
    /// * [`GovernorError::ProposalAlreadyExists`] - If a proposal with the same
    ///   parameters already exists.
    /// * [`GovernorError::InvalidProposalLength`] - If the targets, functions,
    ///   and args vectors have different lengths.
    /// * [`GovernorError::EmptyProposal`] - If the proposal contains no
    ///   actions.
    /// * [`GovernorError::ProposalThresholdNotSet`] - If the proposal threshold
    ///   has not been set.
    /// * [`GovernorError::VotingDelayNotSet`] - If the voting delay has not
    ///   been set.
    /// * [`GovernorError::VotingPeriodNotSet`] - If the voting period has not
    ///   been set.
    ///
    /// # Events
    ///
    /// * topics - `["proposal_created", proposal_id: BytesN<32>, proposer:
    ///   Address]`
    /// * data - `[targets: Vec<Address>, functions: Vec<Symbol>, args:
    ///   Vec<Vec<Val>>, vote_start: u32, vote_end: u32, description: String]`
    ///
    /// # Notes
    ///
    /// * Authorization for `proposer` is required.
    /// * The `proposer` parameter enables flexible access control. The
    ///   implementer can pass any address (e.g., an admin or relayer) to
    ///   customize who is authorized to create proposals.
    fn propose(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description: String,
        proposer: Address,
    ) -> BytesN<32> {
        // Use previous ledger to prevent flash loan based proposals
        let snapshot = e.ledger().sequence() - 1;
        let proposer_votes = Self::get_past_votes(e, proposer.clone(), snapshot);
        storage::propose(e, proposer_votes, targets, functions, args, description, &proposer)
    }

    /// Casts a vote on a proposal and returns the voter's voting power.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    /// * `vote_type` - The type of vote. The interpretation depends on the
    ///   counting module used. For simple counting: 0 = Against, 1 = For, 2 =
    ///   Abstain.
    /// * `reason` - An optional explanation for the vote.
    /// * `voter` - The address casting the vote.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::ProposalNotActive`] - If voting is not currently
    ///   open.
    ///
    /// # Events
    ///
    /// * topics - `["vote_cast", voter: Address, proposal_id: BytesN<32>]`
    /// * data - `[vote_type: u32, weight: u128, reason: String]`
    ///
    /// # Notes
    ///
    /// * Authorization for `voter` is required.
    /// * The `voter` parameter enables flexible access control. The implementer
    ///   can pass any address to customize who is authorized to cast votes
    ///   (e.g., for vote delegation or relaying).
    fn cast_vote(
        e: &Env,
        proposal_id: BytesN<32>,
        vote_type: u32,
        reason: String,
        voter: Address,
    ) -> u128 {
        let snapshot = storage::prepare_vote(e, &voter, &proposal_id);
        let voter_weight = Self::get_past_votes(e, voter.clone(), snapshot);
        Self::count_vote(e, proposal_id.clone(), voter.clone(), vote_type, voter_weight);
        crate::governor::emit_vote_cast(e, &voter, &proposal_id, vote_type, voter_weight, &reason);
        voter_weight
    }

    /// Executes a proposal and returns its unique identifier.
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
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::ProposalNotQueued`] - If the proposal has not been
    ///   queued (only relevant when using queuing extensions).
    /// * [`GovernorError::ProposalNotSuccessful`] - If the proposal has not
    ///   succeeded.
    /// * [`GovernorError::ProposalAlreadyExecuted`] - If the proposal has
    ///   already been executed.
    ///
    /// # Events
    ///
    /// * topics - `["proposal_executed", proposal_id: BytesN<32>]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// * Authorization for `executor` is required.
    /// * The `executor` parameter enables flexible access control. The
    ///   implementer can pass any address to customize who is authorized to
    ///   execute proposals (e.g., restrict to a timelock contract or allow any
    ///   account).
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        executor: Address,
    ) -> BytesN<32> {
        storage::execute(e, targets, functions, args, &description_hash, &executor)
    }

    /// Cancels a proposal and returns its unique identifier.
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
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::ProposalNotCancellable`] - If the proposal is not in
    ///   a cancellable state (Pending, Active, or Succeeded).
    ///
    /// # Events
    ///
    /// * topics - `["proposal_cancelled", proposal_id: BytesN<32>]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// * Authorization for `operator` is required.
    /// * The `operator` parameter enables flexible access control. The
    ///   implementer decides who can cancel proposals (e.g., only the original
    ///   proposer, an admin, or a guardian role).
    fn cancel(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        operator: Address,
    ) -> BytesN<32> {
        storage::cancel(e, targets, functions, args, &description_hash, &operator)
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

/// Time-based proposal states, derived from the current ledger relative to
/// the proposal's voting schedule. These are never set explicitly — they are
/// computed by [`get_proposal_state()`] when no terminal state has been set.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TimeBasedState {
    /// The proposal is pending and voting has not started yet.
    Pending = 0,
    /// The proposal is active and voting is ongoing.
    Active = 1,
    /// The proposal was defeated (did not meet quorum or majority). This is
    /// the default outcome when voting ends and the [`Counting`] module has
    /// not marked the proposal as
    /// [`Succeeded`](ExplicitState::Succeeded).
    Defeated = 2,
}

/// Explicit proposal states, set by the Governor or its extensions.
/// Once an explicit state is stored, it takes precedence over any time-based
/// derivation.
///
/// - [`Canceled`](ExplicitState::Canceled) and
///   [`Executed`](ExplicitState::Executed) are set by the Governor.
/// - [`Succeeded`](ExplicitState::Succeeded) is set by the [`Counting`]
///   module.
/// - [`Queued`](ExplicitState::Queued) and
///   [`Expired`](ExplicitState::Expired) are set by extensions like
///   `TimelockControl`.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExplicitState {
    /// The proposal has been cancelled.
    Canceled = 0,
    /// The proposal succeeded. Set by the [`Counting`] module when the
    /// proposal meets the required quorum and vote thresholds. If a queuing
    /// extension is enabled, this state means the proposal is ready to be
    /// queued.
    Succeeded = 1,
    /// The proposal is queued for execution. Set by extensions like
    /// `TimelockControl`.
    Queued = 2,
    /// The proposal has expired and can no longer be executed. Set by
    /// extensions like `TimelockControl`.
    Expired = 3,
    /// The proposal has been executed.
    Executed = 4,
}

/// The full lifecycle state of a proposal.
///
/// Combines time-based states (derived from the current ledger) and explicit
/// states (stored explicitly). Explicit states take precedence over time-based
/// derivation when present.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalState {
    /// A time-based state derived from the current ledger relative to the
    /// proposal's voting schedule.
    TimeBased(TimeBasedState),
    /// An explicit state set by the Governor or its extensions.
    Explicit(ExplicitState),
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
    /// The proposal has not succeeded.
    ProposalNotSuccessful = 5006,
    /// The proposal has not been queued.
    ProposalNotQueued = 5007,
    /// The proposal has already been executed.
    ProposalAlreadyExecuted = 5008,
    /// The proposal is not in a cancellable state (must be Pending, Active,
    /// or Succeeded).
    ProposalNotCancellable = 5009,
    /// The voting delay has not been set.
    VotingDelayNotSet = 5010,
    /// The voting period has not been set.
    VotingPeriodNotSet = 5011,
    /// The proposal threshold has not been set.
    ProposalThresholdNotSet = 5012,
    /// The name has not been set.
    NameNotSet = 5013,
    /// The version has not been set.
    VersionNotSet = 5014,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for storage entries (in ledgers)
pub const GOVERNOR_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL threshold for extending storage entries (in ledgers)
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
    pub vote_type: u32,
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
    vote_type: u32,
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
