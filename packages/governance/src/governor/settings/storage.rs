//! # Governor Settings Storage Module
//!
//! Storage keys and helper functions for the governor configuration: name,
//! version, voting schedule, proposal threshold, voting token and quorum
//! checkpoints.

use soroban_sdk::{contracttype, panic_with_error, Address, Env, String, Symbol};

use crate::governor::{
    settings::emit_quorum_changed, GovernorError, GOVERNOR_EXTEND_AMOUNT, GOVERNOR_TTL_THRESHOLD,
};

// ################## STORAGE KEYS ##################

/// Storage keys for the governor configuration.
#[derive(Clone)]
#[contracttype]
pub enum GovernorSettingsStorageKey {
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
    /// Number of quorum checkpoints.
    NumQuorumCheckpoints,
    /// Individual quorum checkpoint at index.
    QuorumCheckpoint(u32),
    /// The address of the token contract that implements the Votes trait.
    TokenContract,
}

// ################## STORAGE TYPES ##################

/// A quorum checkpoint recording the quorum value at a specific ledger.
#[derive(Clone)]
#[contracttype]
pub struct QuorumCheckpoint {
    /// The ledger at which this quorum value took effect.
    pub ledger: u32,
    /// The quorum value.
    pub quorum: u128,
}

// ################## QUERY STATE ##################

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
        .get(&GovernorSettingsStorageKey::Name)
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
        .get(&GovernorSettingsStorageKey::Version)
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
        .get(&GovernorSettingsStorageKey::ProposalThreshold)
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
        .get(&GovernorSettingsStorageKey::VotingDelay)
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
        .get(&GovernorSettingsStorageKey::VotingPeriod)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::VotingPeriodNotSet))
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
        .get(&GovernorSettingsStorageKey::TokenContract)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::TokenContractNotSet))
}

/// Returns the quorum value effective at the given ledger.
///
/// The quorum is the minimum total voting power (for + abstain) that must
/// participate for a proposal to be valid. Quorum values are stored as
/// checkpoints, so historical lookups return the value that was in effect
/// at the requested ledger.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `ledger` - The ledger at which to query the quorum.
///
/// # Errors
///
/// * [`GovernorError::QuorumNotSet`] - Occurs if no quorum checkpoint exists at
///   or before the requested ledger.
pub fn get_quorum(e: &Env, ledger: u32) -> u128 {
    let num: u32 =
        e.storage().instance().get(&GovernorSettingsStorageKey::NumQuorumCheckpoints).unwrap_or(0);

    if num == 0 {
        panic_with_error!(e, GovernorError::QuorumNotSet);
    }

    // Check if ledger is at or after the latest checkpoint.
    let latest = get_quorum_checkpoint(e, num - 1);
    if latest.ledger <= ledger {
        return latest.quorum;
    }

    // Check if ledger is before the first checkpoint.
    let first = get_quorum_checkpoint(e, 0);
    if first.ledger > ledger {
        panic_with_error!(e, GovernorError::QuorumNotSet);
    }

    // Binary search for the most recent checkpoint at or before `ledger`.
    let mut low: u32 = 0;
    let mut high: u32 = num - 1;

    while low < high {
        let mid = low + (high - low).div_ceil(2);
        let cp = get_quorum_checkpoint(e, mid);
        if cp.ledger <= ledger {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    get_quorum_checkpoint(e, low).quorum
}

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
    e.storage().instance().set(&GovernorSettingsStorageKey::Name, &name);
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
    e.storage().instance().set(&GovernorSettingsStorageKey::Version, &version);
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
    e.storage().instance().set(&GovernorSettingsStorageKey::ProposalThreshold, &threshold);
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
    e.storage().instance().set(&GovernorSettingsStorageKey::VotingDelay, &delay);
}

/// Sets the voting period.
///
/// A zero period is rejected. It would make `vote_end` equal the vote
/// snapshot, leaving a voting window that no ledger falls inside, so every
/// proposal would move straight from
/// [`crate::governor::ProposalState::Pending`] to
/// [`crate::governor::ProposalState::Defeated`] without ever accepting a vote.
/// Since this setter is typically reachable only through a passing proposal, or
/// only from the constructor, such a governor would in practice be beyond
/// recovery.
///
/// Any non-zero value is accepted, and it remains the responsibility of the
/// implementer to ensure that the period is appropriate (e.g., enough time
/// for voters to participate, but not so long that urgent actions cannot be
/// taken).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `period` - The voting period in ledgers.
///
/// # Errors
///
/// * [`GovernorError::InvalidVotingPeriod`] - Occurs if `period` is zero.
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can call this
/// function.
pub fn set_voting_period(e: &Env, period: u32) {
    if period == 0 {
        panic_with_error!(e, GovernorError::InvalidVotingPeriod);
    }
    e.storage().instance().set(&GovernorSettingsStorageKey::VotingPeriod, &period);
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
    let key = GovernorSettingsStorageKey::TokenContract;
    if e.storage().instance().has(&key) {
        panic_with_error!(e, GovernorError::TokenContractAlreadySet);
    }
    e.storage().instance().set(&key, token_contract);
}

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
    let num: u32 =
        e.storage().instance().get(&GovernorSettingsStorageKey::NumQuorumCheckpoints).unwrap_or(0);
    let ledger = e.ledger().sequence();

    let old_quorum = if num > 0 {
        let last = get_quorum_checkpoint(e, num - 1);
        // If the last checkpoint is at the same ledger, update it in place.
        if last.ledger == ledger {
            e.storage().persistent().set(
                &GovernorSettingsStorageKey::QuorumCheckpoint(num - 1),
                &QuorumCheckpoint { ledger, quorum },
            );
            emit_quorum_changed(e, last.quorum, quorum);
            return;
        }
        last.quorum
    } else {
        0u128
    };

    // Append a new checkpoint.
    e.storage().persistent().set(
        &GovernorSettingsStorageKey::QuorumCheckpoint(num),
        &QuorumCheckpoint { ledger, quorum },
    );
    e.storage().instance().set(&GovernorSettingsStorageKey::NumQuorumCheckpoints, &(num + 1));

    emit_quorum_changed(e, old_quorum, quorum);
}

// ################## LOW-LEVEL HELPERS ##################

/// Returns the quorum checkpoint at the given index.
fn get_quorum_checkpoint(e: &Env, index: u32) -> QuorumCheckpoint {
    let key = GovernorSettingsStorageKey::QuorumCheckpoint(index);
    let cp: QuorumCheckpoint = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, GovernorError::QuorumNotSet));
    e.storage().persistent().extend_ttl(&key, GOVERNOR_TTL_THRESHOLD, GOVERNOR_EXTEND_AMOUNT);
    cp
}
