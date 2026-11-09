//! # Governor Settings Module
//!
//! The configuration of a governor: name, version, voting schedule, proposal
//! threshold, voting token, quorum, counting mode and whether proposals need
//! queueing. See [`GovernorSettings`].

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, String, Symbol};

/// The configuration of a governor: the values that proposal creation, voting
/// and execution are checked against.
///
/// Every method has a default implementation that returns the value stored with
/// the matching setter (e.g. [`crate::governor::set_voting_delay`]). A method
/// can be overridden to compute its value differently (e.g. a proposal
/// threshold relative to the token supply).
/// [`crate::governor::GovernorQueries`] and [`crate::governor::Governor`] read
/// these values through this trait, so an override is what gets enforced.
#[contracttrait]
pub trait GovernorSettings {
    /// Returns the name of the governor.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::NameNotSet`] - Occurs if the name
    ///   has not been set.
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
    /// * [`crate::governor::GovernorError::VersionNotSet`] - Occurs if the
    ///   version has not been set.
    fn version(e: &Env) -> String {
        storage::get_version(e)
    }

    /// Returns the address of the token contract that implements the Votes
    /// trait.
    ///
    /// Proposal creation and vote weighting always read the token from storage.
    /// Overriding this method changes only the address reported to callers, not
    /// the token used for enforcement. To switch the voting token, the stored
    /// value itself has to be updated.
    ///
    /// By design, the library's [`crate::governor::set_token_contract`] can be
    /// called only once, and no function for updating the token afterwards is
    /// provided. Changing the voting token of a live governor is a rare and
    /// sensitive operation, so keeping the token fixed spares users from
    /// unexpected changes to how their votes are counted. For the rare
    /// deployment that does need to switch tokens, a custom setter without the
    /// one-time check can be added to the contract. Gating it behind the
    /// governor's own authorization means a switch can only happen through a
    /// passed proposal:
    ///
    /// ```ignore
    /// use stellar_governance::governor::GovernorSettingsStorageKey;
    ///
    /// #[contractimpl]
    /// impl MyGovernor {
    ///     pub fn update_token_contract(e: &Env, token: Address) {
    ///         e.current_contract_address().require_auth();
    ///         e.storage().instance().set(&GovernorSettingsStorageKey::TokenContract, &token);
    ///     }
    /// }
    /// ```
    ///
    /// Switching the token affects proposals that are still live. Their
    /// proposers were checked against the old token, while their remaining
    /// votes are weighed with the new one, at the proposal's original snapshot
    /// ledger. If the new token has no checkpoints at that ledger, those votes
    /// weigh 0. A switch is therefore best made while no proposal is `Pending`
    /// or `Active`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::TokenContractNotSet`] - Occurs if
    ///   the token contract has not been set.
    fn get_token_contract(e: &Env) -> Address {
        storage::get_token_contract(e)
    }

    /// Returns the voting delay in ledgers.
    ///
    /// The voting delay is the number of ledgers between proposal creation and
    /// the start of voting.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::VotingDelayNotSet`] - Occurs if the
    ///   voting delay has not been set.
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
    /// * [`crate::governor::GovernorError::VotingPeriodNotSet`] - Occurs if the
    ///   voting period has not been set.
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
    /// * [`crate::governor::GovernorError::ProposalThresholdNotSet`] - Occurs
    ///   if the proposal threshold has not been set.
    fn proposal_threshold(e: &Env) -> u128 {
        storage::get_proposal_threshold(e)
    }

    /// Returns the quorum required at the given ledger.
    ///
    /// The quorum only sets **how many** votes a proposal needs. It does not
    /// control:
    ///
    /// * **Which vote types count toward it** (i.e. `for`, `against`,
    ///   `abstain`).
    /// * **The share of votes required for a proposal to pass** (e.g. "at least
    ///   60% of the votes must be `for`").
    ///
    /// Both are decided by
    /// [`crate::governor::GovernorQueries::proposal_succeeded`].
    ///
    /// The default implementation uses checkpoint-based storage, returning the
    /// quorum value that was in effect at the requested `ledger`. Custom
    /// implementations (e.g., fractional quorum based on total supply) may
    /// override this to compute a dynamic quorum.
    ///
    /// # Dynamic Quorum Overrides
    ///
    /// Dynamic quorum implementations (e.g., supply-relative) should typically
    /// **not** use [`crate::governor::set_quorum`] / [`storage::get_quorum`],
    /// as those are designed for the default checkpoint-based fixed quorum.
    /// Instead, compute the quorum from on-chain state at the requested
    /// `ledger`.
    ///
    /// If the dynamic quorum depends on configurable parameters (e.g., a quorum
    /// percentage), those parameters must themselves be queried at the
    /// historical `ledger` — otherwise, later parameter updates would
    /// retroactively change the outcome of existing proposals.
    ///
    /// This method is called with the proposal's `vote_snapshot` ledger, which
    /// may be in the future during the `Pending` state. The override **must not
    /// panic** on future ledger values — if a checkpoint does not yet exist,
    /// return `u128::MAX` so that quorum is unreachable until the real value
    /// becomes available. Quorum is only meaningful after voting ends; during
    /// `Pending` and `Active` states the returned value is unused, so the
    /// `u128::MAX` fallback has no effect on normal operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `ledger` - The ledger number at which to query the quorum.
    ///
    /// # Errors
    ///
    /// * [`crate::governor::GovernorError::QuorumNotSet`] - If no quorum
    ///   checkpoint exists at or before the requested ledger.
    fn quorum(e: &Env, ledger: u32) -> u128 {
        storage::get_quorum(e, ledger)
    }

    /// Returns a symbol identifying the counting strategy.
    ///
    /// This function is expected to be used to display human-readable
    /// information about the counting strategy, for example in UIs.
    ///
    /// For simple counting, this returns `"simple"`. A contract that changes
    /// how votes are counted (e.g. by overriding
    /// [`crate::governor::GovernorQueries::proposal_succeeded`]) should
    /// override this method too.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn counting_mode(e: &Env) -> Symbol {
        storage::counting_mode(e)
    }

    /// Returns whether proposals need to be queued before execution.
    ///
    /// When this returns `false` (the default),
    /// [`crate::governor::Governor::execute`] expects proposals in the
    /// `Succeeded` state and [`crate::governor::Governor::queue`] will revert
    /// with [`crate::governor::GovernorError::QueueNotEnabled`].
    ///
    /// When overridden to return `true`, [`crate::governor::Governor::execute`]
    /// expects proposals in the `Queued` state, meaning
    /// [`crate::governor::Governor::queue`] must be called first to transition
    /// from `Succeeded` to `Queued`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn proposals_need_queuing(_e: &Env) -> bool {
        false
    }
}

// ################## EVENTS ##################

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
