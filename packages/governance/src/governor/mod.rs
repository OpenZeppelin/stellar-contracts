//! # Governor Module
//!
//! Implements on-chain governance functionality for Soroban contracts.
//!
//! This module provides the core governance primitives for decentralized
//! decision-making, including proposal creation, voting, counting, and
//! execution.
//!
//! ## Structure
//!
//! The [`Governor`] trait includes:
//!
//! - Proposal lifecycle management (creation, voting, execution, cancellation)
//! - Vote counting and quorum logic (simple counting by default)
//!
//! The default counting implementation provides **simple counting**:
//!
//! - **Vote types**: Against (0), For (1), Abstain (2)
//! - **Vote success**: `for` votes strictly exceed `against` votes
//! - **Quorum**: Sum of `for` and `abstain` votes meets or exceeds the single
//!   configured quorum value (shared across all proposal tallies)
//!
//! The [`Governor`] trait does not define how to store, manage, and access
//! votes. But Governor trait needs to be able to access the voting power of
//! an account at a specific ledger. [`crate::votes::Votes`] trait is expected
//! to be implemented on a token contract, and the governor contract (which
//! implements [`Governor`] trait) is expected to call the
//! [`crate::votes::Votes`] trait methods on the token contract to access the
//! voting power of an account.
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
//! 3. **Execute**: Successful proposals (meeting quorum and vote thresholds)
//!    can be executed
//! 4. **Cancel**: Proposals can be canceled by the proposer before the voting
//!    period ends
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
//! voting power calculations use
//! [`crate::votes::Votes::get_votes_at_checkpoint()`] which queries the voting
//! power at the snapshot ledger, not the current ledger.
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
//! - **Quorum requirements** ensure a minimum percentage of total voting supply
//!   participates in each proposal
//! - **Voting delay** ([`get_voting_delay()`]) gives token holders time to
//!   acquire more tokens or delegate before voting starts

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, contracttype, Address, BytesN, Env, String,
    Symbol, Val, Vec,
};

pub use crate::governor::storage::{ProposalVoteCounts, VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR};

/// The `Governor` trait defines the core functionality for on-chain governance.
/// It provides a standard interface for creating proposals, counting,
/// and executing approved actions.
///
/// # Default Counting Implementation
///
/// The default implementation provides simple counting with three vote
/// types (Against, For, Abstain), simple majority for success, and a
/// fixed quorum value.
///
/// Implementers can override the counting-related trait methods to provide
/// custom counting strategies (e.g., fractional voting, weighted quorum
/// based on total supply, etc.).
#[contracttrait]
pub trait Governor {
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

    /// Returns the address of the token contract that implements the Votes
    /// trait.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::TokenContractNotSet`] - Occurs if the token contract
    ///   has not been set.
    fn get_token_contract(e: &Env) -> Address {
        storage::get_token_contract(e)
    }

    /// Returns a symbol identifying the counting strategy.
    ///
    /// This function is expected to be used to display human-readable
    /// information about the counting strategy, for example in UIs.
    ///
    /// For simple counting, this returns `"simple"`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn counting_mode(e: &Env) -> Symbol {
        storage::counting_mode(e)
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

    /// Returns the quorum required at the given ledger.
    ///
    /// For simple counting, this returns the configured fixed quorum value
    /// and the `ledger` parameter is ignored. Custom implementations (e.g.,
    /// fractional quorum based on total supply) may use the `ledger`
    /// parameter to compute a dynamic quorum.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `ledger` - The ledger number at which to query the quorum.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::QuorumNotSet`] - If the quorum has not been set.
    fn quorum(e: &Env, ledger: u32) -> u128 {
        let _ = ledger;
        storage::get_quorum(e)
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
    ///
    /// # Note
    ///
    /// Queue logic is expected to be implemented by an extension, so the
    /// default implementation does not handle the `Queued` and `Expired`
    /// states. If you are utilizing an extension that enables
    /// queueing, don't forget to override this function to handle `Queued` and
    /// `Expired` states.
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
    /// * [`GovernorError::MathOverflow`] - If voting schedule calculation
    ///   overflows.
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
        proposer.require_auth();
        storage::propose(e, targets, functions, args, description, &proposer)
    }

    /// Casts a vote on a proposal and returns the voter's voting power.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proposal_id` - The unique identifier of the proposal.
    /// * `vote_type` - The type of vote. For simple counting: 0 = Against, 1 =
    ///   For, 2 = Abstain.
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
        voter.require_auth();
        storage::cast_vote(e, &proposal_id, vote_type, &reason, &voter)
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
        executor.require_auth();

        storage::execute(e, targets, functions, args, &description_hash)
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
    /// * [`GovernorError::ProposalNotCancellable`] - If the proposal is in a
    ///   non-cancellable state (`Canceled`, `Expired`, or `Executed`).
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
        operator.require_auth();

        storage::cancel(e, targets, functions, args, &description_hash)
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
///
/// States are divided into two categories:
///
/// ## Time-based states (derived, never stored explicitly)
///
/// These are computed by [`get_proposal_state()`] from the current ledger
/// relative to the proposal's voting schedule. They are only returned when
/// no explicit state has been set.
///
/// - [`Pending`](ProposalState::Pending) — voting has not started yet.
/// - [`Active`](ProposalState::Active) — voting is ongoing.
/// - [`Defeated`](ProposalState::Defeated) — voting ended **without** the
///   counting logic marking the proposal as `Succeeded`.
///
/// ## Explicit states
///
/// Set explicitly by the Governor or its extensions and persisted in
/// storage. Once set, they take precedence over any time-based derivation.
///
/// - [`Canceled`](ProposalState::Canceled) — set by the Governor.
/// - [`Succeeded`](ProposalState::Succeeded) — set by the counting logic.
/// - [`Queued`](ProposalState::Queued) / [`Expired`](ProposalState::Expired) —
///   set by extensions like `TimelockControl`.
/// - [`Executed`](ProposalState::Executed) — set by the Governor.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ProposalState {
    // ==================== Time-based states ====================
    // Derived from the current ledger.
    /// The proposal is pending and voting has not started yet.
    Pending = 0,
    /// The proposal is active and voting is ongoing.
    Active = 1,
    /// The proposal was defeated (did not meet quorum or majority). This is
    /// the default outcome when voting ends and the counting logic has
    /// not marked the proposal as [`Succeeded`](ProposalState::Succeeded).
    Defeated = 2,

    // ==================== Explicit states ====================
    // Set by the Governor or extensions. Once set, these take precedence
    // over time-based derivation.
    /// The proposal has been cancelled. Set by the Governor.
    Canceled = 3,
    /// The proposal succeeded and can be executed. Set by the counting
    /// logic when the proposal meets the required quorum and vote
    /// thresholds. If a queuing extension is enabled, this state means the
    /// proposal is ready to be queued.
    Succeeded = 4,
    /// The proposal is queued for execution. Set by extensions like
    /// `TimelockControl`.
    Queued = 5,
    /// The proposal has expired and can no longer be executed. Set by
    /// extensions like `TimelockControl`.
    Expired = 6,
    /// The proposal has been executed. Set by the Governor.
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
    /// The proposal has not succeeded.
    ProposalNotSuccessful = 5006,
    /// The proposal has not been queued.
    ProposalNotQueued = 5007,
    /// The proposal has already been executed.
    ProposalAlreadyExecuted = 5008,
    /// The proposal is in a non-cancellable state (`Canceled`, `Expired`, or
    /// `Executed`).
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
    /// Arithmetic overflow occurred.
    MathOverflow = 5015,
    /// The account has already voted on this proposal.
    AlreadyVoted = 5016,
    /// The vote type is invalid (must be 0, 1, or 2).
    InvalidVoteType = 5017,
    /// The quorum has not been set.
    QuorumNotSet = 5018,
    /// The token contract has already been set (can only be initialized once).
    TokenContractAlreadySet = 5019,
    /// The token contract has not been set.
    TokenContractNotSet = 5020,
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

/// Event emitted when the quorum value is changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuorumChanged {
    pub old_quorum: u128,
    pub new_quorum: u128,
}

/// Emits an event when the quorum value is changed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `old_quorum` - The previous quorum value.
/// * `new_quorum` - The new quorum value.
pub fn emit_quorum_changed(e: &Env, old_quorum: u128, new_quorum: u128) {
    QuorumChanged { old_quorum, new_quorum }.publish(e);
}
