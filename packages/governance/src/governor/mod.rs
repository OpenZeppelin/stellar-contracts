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
//! The governor is split into three traits, each building on the previous
//! ones:
//!
//! - [`GovernorSettings`]: configuration values (voting delay and period,
//!   proposal threshold, quorum, voting token, ...).
//! - [`GovernorQueries`]: read-only information about proposals, including
//!   [`GovernorQueries::proposal_state`], which decides the outcome of a vote.
//! - [`Governor`]: the proposal lifecycle (proposing, voting, queueing,
//!   executing and cancelling).
//!
//! A governor contract implements all three. Usually only
//! [`Governor::execute`] and [`Governor::cancel`] need to be written, since
//! every other method has a default implementation:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl GovernorSettings for MyGovernor {}
//!
//! #[contractimpl(contracttrait)]
//! impl GovernorQueries for MyGovernor {}
//!
//! #[contractimpl(contracttrait)]
//! impl Governor for MyGovernor {
//!     fn execute(/* ... */) -> BytesN<32> { /* ... */ }
//!     fn cancel(/* ... */) -> BytesN<32> { /* ... */ }
//! }
//! ```
//!
//! The default implementation provides **simple counting**:
//!
//! - **Vote types**: Against (0), For (1), Abstain (2)
//! - **Vote success**: `for` votes strictly exceed `against` votes
//! - **Quorum**: Sum of `for` and `abstain` votes meets or exceeds the quorum
//!   value in effect at the proposal's `vote_snapshot` ledger (see
//!   [`GovernorSettings::quorum`]). Quorum values are stored as checkpoints, so
//!   updates do not retroactively affect existing proposals
//!
//! Both criteria are checked by [`GovernorQueries::proposal_succeeded`], which
//! can be overridden to change them.
//!
//! The [`Governor`] trait does not define how to store, manage, and access
//! votes. But Governor trait needs to be able to access the voting power of
//! an account at a specific ledger. [`crate::votes::Votes`] trait is expected
//! to be implemented on a token contract, and the governor contract (which
//! implements [`Governor`] trait) is expected to call the
//! [`crate::votes::Votes`] trait methods on the token contract to access the
//! voting power of an account.
//!
//! ## Governance Flow
//!
//! 1. **Propose**: A user with sufficient voting power creates a proposal
//! 2. **Vote**: Token holders vote during the voting period
//! 3. **Execute**: Successful proposals (meeting quorum and vote thresholds)
//!    can be executed
//! 4. **Cancel**: Proposals can be canceled by the proposer unless they are
//!    already Executed, Expired, or Cancelled.
//!
//! When using an extension for `Queue` mechanism, like `TimelockControl`, an
//! additional `Queue` step is added between voting and execution:
//!
//! 1. **Propose** → 2. **Vote** → 3. **Queue** → 4. **Execute**
//!
//! To enable queuing, override [`GovernorSettings::proposals_need_queuing`] to
//! return `true`. This wires up the full queuing flow:
//!
//! - [`Governor::queue`] validates that the  proposal is in the `Succeeded`
//!   state, transitions it to `Queued`, and emits a [`ProposalQueued`] event
//!   with the `eta` (estimated execution ledger).
//! - [`storage::execute`] will then require the proposal to be in the `Queued`
//!   state instead of `Succeeded` before executing.
//!
//! For further customization (e.g. custom delay enforcement), override
//! [`Governor::queue`] and/or [`Governor::execute`] as well.
//!
//! # Security Considerations
//!
//! ## Flash Loan Voting Attack
//!
//! ### Vulnerability Overview
//!
//! A flash loan attack is one where an attacker borrows a large amount of
//! voting tokens, votes on a proposal, and returns the tokens — all within
//! the **same transaction**. Without protection, the attacker would
//! temporarily hold massive voting power at zero cost.
//!
//! ### Mitigation
//!
//! This implementation uses **snapshot-based voting power** with two
//! separate snapshots:
//!
//! **1. Proposer snapshot (`current_ledger - 1`):**
//! When a proposal is created, the proposer's voting power is checked
//! against the **previous** ledger. This prevents an attacker from
//! flash-loaning tokens and creating a proposal in the same transaction.
//!
//! **2. Voter snapshot (`vote_start`):**
//! When a proposal is created, `vote_start` (`current_ledger +
//! voting_delay`) is recorded as the voting power snapshot. When voters
//! cast their votes (at any ledger after `vote_start`), their voting
//! power is looked up at the `vote_start` ledger using
//! [`crate::votes::Votes::get_votes_at_checkpoint()`]. Because
//! checkpoints record the state **after** all transactions in a ledger
//! are finalized, a flash loan that borrows and returns tokens within
//! the same ledger at `vote_start` would show a net-zero balance in the
//! checkpoint — the attack fails.
//!
//! ### Scope and Limitations
//!
//! Snapshot-based voting specifically prevents **same-transaction** (flash
//! loan) attacks. It does **not** prevent an attacker from borrowing tokens
//! across multiple ledgers (e.g., borrowing at ledger N and returning at
//! ledger N+1). Such multi-ledger borrowing carries real economic cost
//! (interest, collateral requirements) and is not considered a flash loan
//! attack. The `voting_delay` parameter gives legitimate token holders time
//! to position themselves after seeing a proposal, which is by design.
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

pub mod queries;
pub mod settings;
pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, contracttype, Address, BytesN, Env, String,
    Symbol, Val, Vec,
};

pub use crate::governor::{
    queries::{
        storage::{
            get_proposal_core, get_proposal_deadline, get_proposal_proposer, get_proposal_snapshot,
            get_proposal_state, get_proposal_vote_counts, has_voted, hash_proposal, quorum_reached,
            tally_succeeded, ProposalCore, ProposalStorageKey, ProposalVoteCounts, VOTE_ABSTAIN,
            VOTE_AGAINST, VOTE_FOR,
        },
        GovernorQueries, GovernorQueriesClient, GovernorQueriesSpec,
    },
    settings::{
        emit_quorum_changed,
        storage::{
            counting_mode, get_name, get_proposal_threshold, get_quorum, get_token_contract,
            get_version, get_voting_delay, get_voting_period, set_name, set_proposal_threshold,
            set_quorum, set_token_contract, set_version, set_voting_delay, set_voting_period,
            GovernorSettingsStorageKey, QuorumCheckpoint,
        },
        GovernorSettings, GovernorSettingsClient, GovernorSettingsSpec, QuorumChanged,
    },
    storage::{cancel, cast_vote, count_vote, execute, propose, queue, ProposalSettings},
};

/// The proposal lifecycle: creating proposals, voting, queueing, executing
/// and cancelling.
///
/// [`Governor::execute`] and [`Governor::cancel`] have no default
/// implementation, because who may execute or cancel a proposal is decided
/// by each contract. Every other method has a default implementation that
/// reads the configuration from [`GovernorSettings`] and the proposal state
/// from [`GovernorQueries::proposal_state`], so overrides of either are
/// respected.
#[contracttrait]
pub trait Governor: GovernorSettings + GovernorQueries {
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
    ///   Vec<Vec<Val>>, vote_snapshot: u32, vote_end: u32, description:
    ///   String]`
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
        let settings = ProposalSettings {
            threshold: Self::proposal_threshold(e),
            voting_delay: Self::voting_delay(e),
            voting_period: Self::voting_period(e),
        };
        storage::propose(e, targets, functions, args, description, &proposer, &settings)
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
    /// * refer to [`GovernorQueries::proposal_state`] errors.
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
        let state = Self::proposal_state(e, proposal_id.clone());
        storage::cast_vote(e, &proposal_id, vote_type, &reason, &voter, state)
    }

    /// Queues a succeeded proposal for execution and returns its unique
    /// identifier.
    ///
    /// This function is only relevant when queuing is enabled, i.e., when
    /// [`GovernorSettings::proposals_need_queuing`] is overridden to return
    /// `true`. If queuing is not enabled, calling this function will revert
    /// with [`GovernorError::QueueNotEnabled`].
    ///
    /// When queuing is enabled, this function transitions a proposal from
    /// the `Succeeded` state to the `Queued` state. The `execute` function
    /// will then require the proposal to be in the `Queued` state before
    /// allowing execution. Note that `eta` enforcement is **not** handled
    /// by the governor itself — it must be enforced by the integration
    /// layer (e.g., a timelock contract that gates execution until the
    /// delay has elapsed).
    ///
    /// # Enabling Queueing
    ///
    /// The default implementation uses **open queueing**: any account can
    /// queue a succeeded proposal without authentication. To enable it,
    /// override [`GovernorSettings::proposals_need_queuing`] to return `true`:
    ///
    /// ```ignore
    /// #[contractimpl(contracttrait)]
    /// impl GovernorSettings for MyGovernor {
    ///     fn proposals_need_queuing(_e: &Env) -> bool {
    ///         true
    ///     }
    /// }
    /// ```
    ///
    /// This is sufficient for the common case — no override of `queue`
    /// itself is needed.
    ///
    /// # Restricting Access
    ///
    /// If you need to restrict who can queue proposals, override this
    /// method with custom access control logic. Call [`storage::queue`]
    /// after your checks:
    ///
    /// ```ignore
    /// #[contractimpl(contracttrait)]
    /// impl Governor for MyGovernor {
    ///     // Restricted — only a timelock contract can queue (queueing must also
    ///     // be enabled in `GovernorSettings`, as shown above):
    ///     fn queue(
    ///         e: &Env,
    ///         targets: Vec<Address>,
    ///         functions: Vec<Symbol>,
    ///         args: Vec<Vec<Val>>,
    ///         description_hash: BytesN<32>,
    ///         eta: u32,
    ///         operator: Address,
    ///     ) -> BytesN<32> {
    ///         let timelock = storage::get_timelock(e);
    ///         assert!(operator == timelock);
    ///         operator.require_auth();
    ///         let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
    ///         let state = Self::proposal_state(e, proposal_id);
    ///         storage::queue(e, targets, functions, args, &description_hash, eta, state)
    ///     }
    /// }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `targets` - The addresses of contracts to call.
    /// * `functions` - The function names to invoke on each target.
    /// * `args` - The arguments for each function call.
    /// * `description_hash` - The keccak256 hash of the description's raw
    ///   bytes.
    /// * `eta` - The estimated ledger sequence for execution. Emitted in the
    ///   event only; not stored or enforced by the governor. Typically computed
    ///   as `current_ledger + timelock_delay`.
    /// * `operator` - The address queuing the proposal. Unused in the default
    ///   (open) implementation, but available for access-control overrides.
    ///
    /// # Errors
    ///
    /// * [`GovernorError::ProposalNotFound`] - If the proposal does not exist.
    /// * [`GovernorError::QueueNotEnabled`] - If queuing is not enabled (i.e.,
    ///   [`GovernorSettings::proposals_need_queuing`] returns `false`).
    /// * [`GovernorError::ProposalNotSuccessful`] - If the proposal has not
    ///   succeeded.
    /// * refer to [`GovernorQueries::proposal_state`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["proposal_queued", proposal_id: BytesN<32>]`
    /// * data - `[eta: u32]`
    fn queue(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        eta: u32,
        _operator: Address,
    ) -> BytesN<32> {
        if !Self::proposals_need_queuing(e) {
            soroban_sdk::panic_with_error!(e, GovernorError::QueueNotEnabled);
        }
        let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
        let state = Self::proposal_state(e, proposal_id);
        storage::queue(e, targets, functions, args, &description_hash, eta, state)
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
    /// # IMPLEMENTATION REQUIRED — ACCESS CONTROL
    ///
    /// **This function has no default implementation.** The implementer MUST
    /// define who is authorized to execute proposals. Consider the following:
    ///
    /// - **Open execution**: Allow anyone to trigger execution of a succeeded
    ///   proposal. In this case, `executor.require_auth()` is unnecessary since
    ///   the `executor` parameter serves no access-control purpose.
    /// - **Restricted execution**: Restrict execution to a specific role (e.g.,
    ///   a timelock contract, an admin, or the original proposer). Validate
    ///   `executor` against the allowed role and call `executor.require_auth()`
    ///   explicitly if needed.
    ///
    /// [`storage::execute`] is suggested to perform the actual state
    /// transition and cross-contract calls after access control and
    /// authorization logic has been applied.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Open execution — anyone can trigger a succeeded proposal:
    /// fn execute(e: &Env, targets: Vec<Address>, /* ... */) -> BytesN<32> {
    ///     let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
    ///     let state = Self::proposal_state(e, proposal_id);
    ///     storage::execute(e, targets, functions, args, &description_hash, Self::proposals_need_queuing(e), state)
    /// }
    ///
    /// // Restricted — only a timelock contract can execute:
    /// fn execute(e: &Env, targets: Vec<Address>, /* ... */) -> BytesN<32> {
    ///     let timelock = storage::get_timelock(e);
    ///     assert!(executor == timelock);
    ///     executor.require_auth();
    ///     let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
    ///     let state = Self::proposal_state(e, proposal_id);
    ///     storage::execute(e, targets, functions, args, &description_hash, Self::proposals_need_queuing(e), state)
    /// }
    ///
    /// // Role-based — using the `stellar-macros` access control macro:
    /// #[only_role(executor, "executor")]
    /// fn execute(e: &Env, targets: Vec<Address>, /* ... */) -> BytesN<32> {
    ///     let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
    ///     let state = Self::proposal_state(e, proposal_id);
    ///     storage::execute(e, targets, functions, args, &description_hash, Self::proposals_need_queuing(e), state)
    /// }
    /// ```
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        executor: Address,
    ) -> BytesN<32>;

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
    /// # IMPLEMENTATION REQUIRED — ACCESS CONTROL
    ///
    /// **This function has no default implementation.** The implementer MUST
    /// define who is authorized to cancel proposals. Consider the following:
    ///
    /// - **Proposer-only cancellation**: Only the original proposer can cancel.
    ///   Validate `operator` against the stored proposer and call
    ///   `operator.require_auth()` explicitly if needed.
    /// - **Guardian/admin cancellation**: A privileged role (e.g., guardian or
    ///   admin) can cancel any proposal. Validate `operator` against the role
    ///   and call `operator.require_auth()` explicitly if needed.
    ///
    /// [`storage::cancel`] is suggested to perform the actual state transition
    /// after access control and authorization logic has been applied.
    ///
    /// # Note
    ///
    /// [`storage::cancel`] only updates the governor-level proposal state. If
    /// the proposal has already been queued in an external timelock, the
    /// implementer must also cancel the corresponding timelock operation
    /// (e.g. via [`crate::timelock::cancel_operation`])
    /// to prevent it from remaining executable through the timelock directly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Only the original proposer can cancel:
    /// fn cancel(e: &Env, targets: Vec<Address>, /* ... */) -> BytesN<32> {
    ///     let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
    ///     let proposer = storage::get_proposal_proposer(e, &proposal_id);
    ///     assert!(operator == proposer);
    ///     operator.require_auth();
    ///     storage::cancel(e, targets, functions, args, &description_hash)
    /// }
    ///
    /// // Role-based — using the `stellar-macros` access control macro:
    /// #[only_role(operator, "canceller")]
    /// fn cancel(e: &Env, targets: Vec<Address>, /* ... */) -> BytesN<32> {
    ///     storage::cancel(e, targets, functions, args, &description_hash)
    /// }
    /// ```
    fn cancel(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        operator: Address,
    ) -> BytesN<32>;
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
    ProposalNotFound = 4200,
    /// The proposal already exists.
    ProposalAlreadyExists = 4201,
    /// The proposer does not have enough voting power.
    InsufficientProposerVotes = 4202,
    /// The proposal contains no actions.
    EmptyProposal = 4203,
    /// The targets, functions, and args vectors have different lengths.
    InvalidProposalLength = 4204,
    /// The proposal is not in the active state.
    ProposalNotActive = 4205,
    /// The proposal has not succeeded.
    ProposalNotSuccessful = 4206,
    /// The proposal has not been queued.
    ProposalNotQueued = 4207,
    /// The proposal has already been executed.
    ProposalAlreadyExecuted = 4208,
    /// The proposal is in a non-cancellable state (`Canceled`, `Expired`, or
    /// `Executed`).
    ProposalNotCancellable = 4209,
    /// The voting delay has not been set.
    VotingDelayNotSet = 4210,
    /// The voting period has not been set.
    VotingPeriodNotSet = 4211,
    /// The proposal threshold has not been set.
    ProposalThresholdNotSet = 4212,
    /// The name has not been set.
    NameNotSet = 4213,
    /// The version has not been set.
    VersionNotSet = 4214,
    /// Arithmetic overflow occurred.
    MathOverflow = 4215,
    /// The account has already voted on this proposal.
    AlreadyVoted = 4216,
    /// The vote type is invalid (must be 0, 1, or 2).
    InvalidVoteType = 4217,
    /// The quorum has not been set.
    QuorumNotSet = 4218,
    /// The token contract has already been set (can only be initialized once).
    TokenContractAlreadySet = 4219,
    /// The token contract has not been set.
    TokenContractNotSet = 4220,
    /// The proposal description exceeds the maximum allowed length.
    DescriptionTooLong = 4221,
    /// Queuing is not enabled for this governor.
    QueueNotEnabled = 4222,
    /// The voting period is zero, which would leave every proposal unvotable.
    InvalidVotingPeriod = 4223,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for storage entries (in ledgers)
pub const GOVERNOR_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL threshold for extending storage entries (in ledgers)
pub const GOVERNOR_TTL_THRESHOLD: u32 = GOVERNOR_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Maximum allowed length (in bytes) for a proposal description.
pub const MAX_DESCRIPTION_LENGTH: u32 = 4096;

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
    pub vote_snapshot: u32,
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
/// * `vote_snapshot` - The ledger at which voting power is snapshotted. Voting
///   opens on the next ledger (`vote_snapshot + 1`).
/// * `vote_end` - The last ledger where voting is active (inclusive).
/// * `description` - The proposal description.
#[allow(clippy::too_many_arguments)]
pub fn emit_proposal_created(
    e: &Env,
    proposal_id: &BytesN<32>,
    proposer: &Address,
    targets: &Vec<Address>,
    functions: &Vec<Symbol>,
    args: &Vec<Vec<Val>>,
    vote_snapshot: u32,
    vote_end: u32,
    description: &String,
) {
    ProposalCreated {
        proposal_id: proposal_id.clone(),
        proposer: proposer.clone(),
        targets: targets.clone(),
        functions: functions.clone(),
        args: args.clone(),
        vote_snapshot,
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
    pub eta: u32,
}

/// Emits an event when a proposal is queued.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `proposal_id` - The unique identifier of the proposal.
/// * `eta` - The estimated ledger sequence for execution. Informational only;
///   not enforced by the governor.
pub fn emit_proposal_queued(e: &Env, proposal_id: &BytesN<32>, eta: u32) {
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
