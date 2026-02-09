use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::votes::{
    emit_delegate_changed, emit_delegate_votes_changed, VotesError, VOTES_EXTEND_AMOUNT,
    VOTES_TTL_THRESHOLD,
};

// ################## ENUMS ##################

/// Represents the direction of a checkpoint delta operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CheckpointOp {
    /// Add the delta to the previous value (e.g., minting, receiving votes).
    Add,
    /// Subtract the delta from the previous value (e.g., burning, losing
    /// votes).
    Sub,
}

/// Selects the checkpoint timeline to operate on.
///
/// Each variant maps to a different set of storage keys so that
/// per-account voting-power history and aggregate total-supply history
/// are kept in separate checkpoint sequences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CheckpointType {
    /// The global total-supply checkpoint timeline.
    TotalSupply,
    /// A per-account (delegate) voting-power checkpoint timeline.
    Account(Address),
}

// ################## TYPES ##################

/// A checkpoint recording voting power at a specific timepoint.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    /// The timepoint when this checkpoint was created
    pub timestamp: u64,
    /// The voting power at this timepoint
    pub votes: u128,
}

/// Storage keys for the votes module.
///
/// Only delegated voting power counts as votes (i.e., only delegatees can
/// vote), so the storage design tracks delegates and their checkpointed
/// voting power separately from the raw voting units held by each account.
#[derive(Clone)]
#[contracttype]
pub enum VotesStorageKey {
    /// Maps account to its delegate
    Delegatee(Address),
    /// Number of checkpoints for a delegate
    NumCheckpoints(Address),
    /// Individual checkpoint for a delegate at index
    DelegateCheckpoint(Address, u32),
    /// Number of total supply checkpoints
    NumTotalSupplyCheckpoints,
    /// Individual total supply checkpoint at index
    TotalSupplyCheckpoint(u32),
    /// Voting units held by an account (tracked separately from delegation)
    VotingUnits(Address),
}

// ################## QUERY STATE ##################

/// Returns the current voting power (delegated votes) of an account.
///
/// This is the total voting power delegated to this account by others
/// (and itself if self-delegated). Returns `0` if no voting power has been
/// delegated to this account, or if the account does not exist in the
/// contract.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query voting power for.
pub fn get_votes(e: &Env, account: &Address) -> u128 {
    let cp_type = CheckpointType::Account(account.clone());
    let num = get_num_checkpoints(e, &cp_type);
    if num == 0 {
        return 0;
    }
    get_checkpoint(e, &cp_type, num - 1).votes
}

/// Returns the voting power (delegated votes) of an account at a specific
/// past timepoint.
///
/// Returns `0` if no voting power had been delegated to this account at the
/// given timepoint, or if the account does not exist in the contract.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query voting power for.
/// * `timepoint` - The timepoint to query (must be in the past).
///
/// # Errors
///
/// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
pub fn get_votes_at_checkpoint(e: &Env, account: &Address, timepoint: u64) -> u128 {
    if timepoint >= e.ledger().timestamp() {
        panic_with_error!(e, VotesError::FutureLookup);
    }

    let cp_type = CheckpointType::Account(account.clone());
    let num = get_num_checkpoints(e, &cp_type);
    if num == 0 {
        return 0;
    }

    lookup_checkpoint_at(e, timepoint, num, &cp_type)
}

/// Returns the current total supply of voting units.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
pub fn get_total_supply(e: &Env) -> u128 {
    let cp_type = CheckpointType::TotalSupply;
    let num = get_num_checkpoints(e, &cp_type);
    if num == 0 {
        return 0;
    }
    get_checkpoint(e, &cp_type, num - 1).votes
}

/// Returns the total supply of voting units at a specific past timepoint.
///
/// Returns `0` if there were no voting units at the given timepoint.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `timepoint` - The timepoint to query (must be in the past).
///
/// # Errors
///
/// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
pub fn get_past_total_supply(e: &Env, timepoint: u64) -> u128 {
    if timepoint >= e.ledger().timestamp() {
        panic_with_error!(e, VotesError::FutureLookup);
    }

    let cp_type = CheckpointType::TotalSupply;
    let num = get_num_checkpoints(e, &cp_type);
    if num == 0 {
        return 0;
    }

    lookup_checkpoint_at(e, timepoint, num, &cp_type)
}

/// Returns the delegate for an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query the delegate for.
///
/// # Returns
///
/// * `Some(Address)` - The delegate address if delegation is set.
/// * `None` - If the account has not delegated.
pub fn get_delegate(e: &Env, account: &Address) -> Option<Address> {
    let key = VotesStorageKey::Delegatee(account.clone());
    if let Some(delegatee) = e.storage().persistent().get::<_, Address>(&key) {
        e.storage().persistent().extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
        Some(delegatee)
    } else {
        None
    }
}

/// Returns the number of checkpoints for an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query checkpoints for.
pub fn num_checkpoints(e: &Env, account: &Address) -> u32 {
    get_num_checkpoints(e, &CheckpointType::Account(account.clone()))
}

/// Returns the voting units held by an account.
///
/// Voting units represent the underlying balance that can be delegated.
/// This is tracked separately from the delegated voting power.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query voting units for.
pub fn get_voting_units(e: &Env, account: &Address) -> u128 {
    let key = VotesStorageKey::VotingUnits(account.clone());
    if let Some(units) = e.storage().persistent().get::<_, u128>(&key) {
        e.storage().persistent().extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
        units
    } else {
        0
    }
}

// ################## CHANGE STATE ##################

/// Delegates voting power from `account` to `delegatee`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account delegating its voting power.
/// * `delegatee` - The account receiving the delegated voting power.
///
/// # Events
///
/// * topics - `["DelegateChanged", delegator: Address]`
/// * data - `[from_delegate: Option<Address>, to_delegate: Address]`
///
/// * topics - `["DelegateVotesChanged", delegate: Address]`
/// * data - `[old_votes: u128, new_votes: u128]`
///
/// # Errors
///
/// * [`VotesError::SameDelegate`] - If `delegatee` is already the
///   current delegate for `account`.
///
/// # Notes
///
/// Authorization for `account` is required.
pub fn delegate(e: &Env, account: &Address, delegatee: &Address) {
    account.require_auth();
    let old_delegate = get_delegate(e, account);

    if old_delegate.as_ref() == Some(delegatee) {
        panic_with_error!(e, VotesError::SameDelegate);
    }

    e.storage().persistent().set(&VotesStorageKey::Delegatee(account.clone()), delegatee);

    emit_delegate_changed(e, account, old_delegate.clone(), delegatee);

    let voting_units = get_voting_units(e, account);
    move_delegate_votes(e, old_delegate.as_ref(), Some(delegatee), voting_units);
}

/// Transfers voting units between accounts.
///
/// This function should be called by the token contract whenever tokens
/// are transferred, minted, or burned. It updates the voting power of
/// the delegates accordingly.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The source account (`None` for minting).
/// * `to` - The destination account (`None` for burning).
/// * `amount` - The amount of voting units to transfer.
///
/// # Events
///
/// * topics - `["DelegateVotesChanged", delegate: Address]`
/// * data - `[previous_votes: u128, new_votes: u128]`
///
/// # Notes
///
/// This function does not perform authorization - it should be called
/// from within the token contract's transfer/mint/burn logic.
pub fn transfer_voting_units(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: u128) {
    if amount == 0 {
        return;
    }

    // Look up delegates first so we can make a single move_delegate_votes call
    let from_delegate = from.and_then(|addr| get_delegate(e, addr));
    let to_delegate = to.and_then(|addr| get_delegate(e, addr));

    if let Some(from_addr) = from {
        let from_units = get_voting_units(e, from_addr);
        let Some(new_from_units) = from_units.checked_sub(amount) else {
            panic_with_error!(e, VotesError::InsufficientVotingUnits);
        };
        set_voting_units(e, from_addr, new_from_units);
    } else {
        // Minting: increase total supply
        push_checkpoint(e, &CheckpointType::TotalSupply, CheckpointOp::Add, amount);
    }

    if let Some(to_addr) = to {
        let to_units = get_voting_units(e, to_addr);
        let Some(new_to_units) = to_units.checked_add(amount) else {
            panic_with_error!(e, VotesError::MathOverflow);
        };
        set_voting_units(e, to_addr, new_to_units);
    } else {
        // Burning: decrease total supply
        push_checkpoint(e, &CheckpointType::TotalSupply, CheckpointOp::Sub, amount);
    }

    move_delegate_votes(e, from_delegate.as_ref(), to_delegate.as_ref(), amount);
}

// ################## INTERNAL HELPERS ##################

/// Sets the voting units for an account.
fn set_voting_units(e: &Env, account: &Address, units: u128) {
    let key = VotesStorageKey::VotingUnits(account.clone());
    if units == 0 {
        e.storage().persistent().remove(&key);
    } else {
        e.storage().persistent().set(&key, &units);
    }
}

/// Moves delegated votes from one delegate to another.
fn move_delegate_votes(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: u128) {
    if amount == 0 {
        return;
    }

    if from == to {
        return;
    }

    if let Some(from_addr) = from {
        let cp_type = CheckpointType::Account(from_addr.clone());
        let (old_votes, new_votes) = push_checkpoint(e, &cp_type, CheckpointOp::Sub, amount);
        emit_delegate_votes_changed(e, from_addr, old_votes, new_votes);
    }

    if let Some(to_addr) = to {
        let cp_type = CheckpointType::Account(to_addr.clone());
        let (old_votes, new_votes) = push_checkpoint(e, &cp_type, CheckpointOp::Add, amount);
        emit_delegate_votes_changed(e, to_addr, old_votes, new_votes);
    }
}

/// Binary search over checkpoints to find votes at a given timepoint.
///
/// `num` must be > 0.
fn lookup_checkpoint_at(
    e: &Env,
    timepoint: u64,
    num: u32,
    checkpoint_type: &CheckpointType,
) -> u128 {
    // Check if timepoint is after the latest checkpoint
    let latest = get_checkpoint(e, checkpoint_type, num - 1);
    if latest.timestamp <= timepoint {
        return latest.votes;
    }

    // Check if timepoint is before the first checkpoint
    let first = get_checkpoint(e, checkpoint_type, 0);
    if first.timestamp > timepoint {
        return 0;
    }

    // Binary search
    let mut low: u32 = 0;
    let mut high: u32 = num - 1;

    while low < high {
        let mid = (low + high).div_ceil(2);
        let checkpoint = get_checkpoint(e, checkpoint_type, mid);
        if checkpoint.timestamp <= timepoint {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    get_checkpoint(e, checkpoint_type, low).votes
}

/// Applies a [`CheckpointOp`] to compute the new votes value from the
/// previous value and a delta.
fn apply_checkpoint_op(e: &Env, previous: u128, op: CheckpointOp, delta: u128) -> u128 {
    match op {
        CheckpointOp::Add => previous
            .checked_add(delta)
            .unwrap_or_else(|| panic_with_error!(e, VotesError::MathOverflow)),
        CheckpointOp::Sub => previous
            .checked_sub(delta)
            .unwrap_or_else(|| panic_with_error!(e, VotesError::MathOverflow)),
    }
}

/// Returns the storage key for a checkpoint at the given index.
fn checkpoint_storage_key(checkpoint_type: &CheckpointType, index: u32) -> VotesStorageKey {
    match checkpoint_type {
        CheckpointType::TotalSupply => VotesStorageKey::TotalSupplyCheckpoint(index),
        CheckpointType::Account(account) => {
            VotesStorageKey::DelegateCheckpoint(account.clone(), index)
        }
    }
}

/// Returns the number of checkpoints for the given checkpoint type.
fn get_num_checkpoints(e: &Env, checkpoint_type: &CheckpointType) -> u32 {
    match checkpoint_type {
        CheckpointType::TotalSupply => {
            let key = VotesStorageKey::NumTotalSupplyCheckpoints;
            e.storage().instance().get(&key).unwrap_or(0)
        }
        CheckpointType::Account(account) => {
            let key = VotesStorageKey::NumCheckpoints(account.clone());
            if let Some(checkpoints) = e.storage().persistent().get::<_, u32>(&key) {
                e.storage()
                    .persistent()
                    .extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
                checkpoints
            } else {
                0
            }
        }
    }
}

/// Gets a checkpoint at a specific index for the given checkpoint type.
fn get_checkpoint(e: &Env, checkpoint_type: &CheckpointType, index: u32) -> Checkpoint {
    let key = checkpoint_storage_key(checkpoint_type, index);
    if let Some(checkpoint) = e.storage().persistent().get::<_, Checkpoint>(&key) {
        e.storage().persistent().extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
        checkpoint
    } else {
        Checkpoint { timestamp: 0, votes: 0 }
    }
}

/// Pushes a new checkpoint or updates the last one if same timepoint.
/// Returns (previous_votes, new_votes).
fn push_checkpoint(
    e: &Env,
    checkpoint_type: &CheckpointType,
    op: CheckpointOp,
    delta: u128,
) -> (u128, u128) {
    let num = get_num_checkpoints(e, checkpoint_type);
    let timestamp = e.ledger().timestamp();

    let previous_votes =
        if num > 0 { get_checkpoint(e, checkpoint_type, num - 1).votes } else { 0 };
    let votes = apply_checkpoint_op(e, previous_votes, op, delta);

    // Check if we can update the last checkpoint (same timepoint)
    if num > 0 {
        let last_checkpoint = get_checkpoint(e, checkpoint_type, num - 1);
        if last_checkpoint.timestamp == timestamp {
            let key = checkpoint_storage_key(checkpoint_type, num - 1);
            e.storage()
                .persistent()
                .set(&key, &Checkpoint { timestamp, votes });
            return (previous_votes, votes);
        }
    }

    // Create new checkpoint
    let key = checkpoint_storage_key(checkpoint_type, num);
    e.storage().persistent().set(&key, &Checkpoint { timestamp, votes });

    // Update checkpoint count
    match checkpoint_type {
        CheckpointType::TotalSupply => {
            let num_key = VotesStorageKey::NumTotalSupplyCheckpoints;
            e.storage().instance().set(&num_key, &(num + 1));
        }
        CheckpointType::Account(account) => {
            let num_key = VotesStorageKey::NumCheckpoints(account.clone());
            e.storage().persistent().set(&num_key, &(num + 1));
        }
    }

    (previous_votes, votes)
}
