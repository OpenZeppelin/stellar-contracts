use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::votes::{
    emit_delegate_changed, emit_delegate_votes_changed, VotesError, VOTES_EXTEND_AMOUNT,
    VOTES_TTL_THRESHOLD,
};

// ################## TYPES ##################

/// A checkpoint recording voting power at a specific timestamp.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    /// The timestamp when this checkpoint was created
    pub timestamp: u64,
    /// The voting power at this timestamp
    pub votes: u128,
}

/// Storage keys for the votes module.
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

/// Returns the current voting power of an account.
///
/// This is the total voting power delegated to this account by others
/// (and itself if self-delegated).
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query voting power for.
pub fn get_votes(e: &Env, account: &Address) -> u128 {
    let num = num_checkpoints(e, account);
    if num == 0 {
        return 0;
    }
    get_checkpoint(e, account, num - 1).votes
}

/// Returns the voting power of an account at a specific past timestamp.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The address to query voting power for.
/// * `timepoint` - The timestamp to query (must be in the past).
///
/// # Errors
///
/// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
pub fn get_past_votes(e: &Env, account: &Address, timepoint: u64) -> u128 {
    if timepoint >= e.ledger().timestamp() {
        panic_with_error!(e, VotesError::FutureLookup);
    }

    let num = num_checkpoints(e, account);
    if num == 0 {
        return 0;
    }

    // Check if timepoint is after the latest checkpoint
    let latest = get_checkpoint(e, account, num - 1);
    if latest.timestamp <= timepoint {
        return latest.votes;
    }

    // Check if timepoint is before the first checkpoint
    let first = get_checkpoint(e, account, 0);
    if first.timestamp > timepoint {
        return 0;
    }

    // Binary search
    let mut low: u32 = 0;
    let mut high: u32 = num - 1;

    while low < high {
        let mid = (low + high).div_ceil(2);
        let checkpoint = get_checkpoint(e, account, mid);
        if checkpoint.timestamp <= timepoint {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    get_checkpoint(e, account, low).votes
}

/// Returns the current total supply of voting units.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
pub fn get_total_supply(e: &Env) -> u128 {
    let num = num_total_supply_checkpoints(e);
    if num == 0 {
        return 0;
    }
    get_total_supply_checkpoint(e, num - 1).votes
}

/// Returns the total supply of voting units at a specific past timestamp.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `timepoint` - The timestamp to query (must be in the past).
///
/// # Errors
///
/// * [`VotesError::FutureLookup`] - If `timepoint` >= current timestamp.
pub fn get_past_total_supply(e: &Env, timepoint: u64) -> u128 {
    if timepoint >= e.ledger().timestamp() {
        panic_with_error!(e, VotesError::FutureLookup);
    }

    let num = num_total_supply_checkpoints(e);
    if num == 0 {
        return 0;
    }

    // Check if timepoint is after the latest checkpoint
    let latest = get_total_supply_checkpoint(e, num - 1);
    if latest.timestamp <= timepoint {
        return latest.votes;
    }

    // Check if timepoint is before the first checkpoint
    let first = get_total_supply_checkpoint(e, 0);
    if first.timestamp > timepoint {
        return 0;
    }

    // Binary search
    let mut low: u32 = 0;
    let mut high: u32 = num - 1;

    while low < high {
        let mid = (low + high).div_ceil(2);
        let checkpoint = get_total_supply_checkpoint(e, mid);
        if checkpoint.timestamp <= timepoint {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    get_total_supply_checkpoint(e, low).votes
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
    let key = VotesStorageKey::NumCheckpoints(account.clone());
    if let Some(checkpoints) = e.storage().persistent().get::<_, u32>(&key) {
        e.storage().persistent().extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
        checkpoints
    } else {
        0
    }
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
/// * [`DelegateChanged`] - Emitted when delegation changes.
/// * [`DelegateVotesChanged`] - Emitted for both old and new delegates if their
///   voting power changes.
///
/// # Notes
///
/// Authorization for `account` is required.
pub fn delegate(e: &Env, account: &Address, delegatee: &Address) {
    account.require_auth();
    let old_delegate = get_delegate(e, account);

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
/// # Notes
///
/// This function does not perform authorization - it should be called
/// from within the token contract's transfer/mint/burn logic.
pub fn transfer_voting_units(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: u128) {
    if amount == 0 {
        return;
    }

    if let Some(from_addr) = from {
        let from_units = get_voting_units(e, from_addr);
        let Some(new_from_units) = from_units.checked_sub(amount) else {
            panic_with_error!(e, VotesError::InsufficientVotingUnits);
        };
        set_voting_units(e, from_addr, new_from_units);

        let from_delegate = get_delegate(e, from_addr);
        move_delegate_votes(e, from_delegate.as_ref(), None, amount);
    } else {
        // Minting: increase total supply
        push_total_supply_checkpoint(e, true, amount);
    }

    if let Some(to_addr) = to {
        let to_units = get_voting_units(e, to_addr);
        let Some(new_to_units) = to_units.checked_add(amount) else {
            panic_with_error!(e, VotesError::MathOverflow);
        };
        set_voting_units(e, to_addr, new_to_units);

        let to_delegate = get_delegate(e, to_addr);
        move_delegate_votes(e, None, to_delegate.as_ref(), amount);
    } else {
        // Burning: decrease total supply
        push_total_supply_checkpoint(e, false, amount);
    }
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
        let (old_votes, new_votes) = push_checkpoint(e, from_addr, false, amount);
        emit_delegate_votes_changed(e, from_addr, old_votes, new_votes);
    }

    if let Some(to_addr) = to {
        let (old_votes, new_votes) = push_checkpoint(e, to_addr, true, amount);
        emit_delegate_votes_changed(e, to_addr, old_votes, new_votes);
    }
}

/// Gets a checkpoint for a delegate at a specific index.
fn get_checkpoint(e: &Env, account: &Address, index: u32) -> Checkpoint {
    let key = VotesStorageKey::DelegateCheckpoint(account.clone(), index);
    if let Some(checkpoint) = e.storage().persistent().get::<_, Checkpoint>(&key) {
        e.storage().persistent().extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
        checkpoint
    } else {
        Checkpoint { timestamp: 0, votes: 0 }
    }
}

/// Pushes a new checkpoint or updates the last one if same timestamp.
/// Returns (last_votes, new_votes).
fn push_checkpoint(e: &Env, account: &Address, add: bool, delta: u128) -> (u128, u128) {
    let num = num_checkpoints(e, account);
    let current_timestamp = e.ledger().timestamp();

    let last_votes = if num > 0 { get_checkpoint(e, account, num - 1).votes } else { 0 };

    let votes = if add {
        last_votes
            .checked_add(delta)
            .unwrap_or_else(|| panic_with_error!(e, VotesError::MathOverflow))
    } else {
        last_votes
            .checked_sub(delta)
            .unwrap_or_else(|| panic_with_error!(e, VotesError::MathOverflow))
    };

    // Check if we can update the last checkpoint (same timestamp)
    if num > 0 {
        let last_checkpoint = get_checkpoint(e, account, num - 1);
        if last_checkpoint.timestamp == current_timestamp {
            // Update existing checkpoint
            let key = VotesStorageKey::DelegateCheckpoint(account.clone(), num - 1);
            e.storage().persistent().set(&key, &Checkpoint { timestamp: current_timestamp, votes });
            return (last_votes, votes);
        }
    }

    // Create new checkpoint
    let key = VotesStorageKey::DelegateCheckpoint(account.clone(), num);
    e.storage().persistent().set(&key, &Checkpoint { timestamp: current_timestamp, votes });

    // Update checkpoint count
    let num_key = VotesStorageKey::NumCheckpoints(account.clone());
    e.storage().persistent().set(&num_key, &(num + 1));

    (last_votes, votes)
}

// ################## TOTAL SUPPLY CHECKPOINTS ##################

/// Returns the number of total supply checkpoints.
fn num_total_supply_checkpoints(e: &Env) -> u32 {
    let key = VotesStorageKey::NumTotalSupplyCheckpoints;
    e.storage().instance().get(&key).unwrap_or(0)
}

/// Gets a total supply checkpoint at a specific index.
fn get_total_supply_checkpoint(e: &Env, index: u32) -> Checkpoint {
    let key = VotesStorageKey::TotalSupplyCheckpoint(index);
    if let Some(checkpoint) = e.storage().persistent().get::<_, Checkpoint>(&key) {
        e.storage().persistent().extend_ttl(&key, VOTES_TTL_THRESHOLD, VOTES_EXTEND_AMOUNT);
        checkpoint
    } else {
        Checkpoint { timestamp: 0, votes: 0 }
    }
}

/// Pushes a new total supply checkpoint or updates the last one if same
/// timestamp.
fn push_total_supply_checkpoint(e: &Env, add: bool, delta: u128) {
    let num = num_total_supply_checkpoints(e);
    let current_timestamp = e.ledger().timestamp();

    let last_votes = if num > 0 { get_total_supply_checkpoint(e, num - 1).votes } else { 0 };

    let votes = if add {
        last_votes
            .checked_add(delta)
            .unwrap_or_else(|| panic_with_error!(e, VotesError::MathOverflow))
    } else {
        last_votes
            .checked_sub(delta)
            .unwrap_or_else(|| panic_with_error!(e, VotesError::MathOverflow))
    };

    // Check if we can update the last checkpoint (same timestamp)
    if num > 0 {
        let last_checkpoint = get_total_supply_checkpoint(e, num - 1);
        if last_checkpoint.timestamp == current_timestamp {
            let key = VotesStorageKey::TotalSupplyCheckpoint(num - 1);
            e.storage().persistent().set(&key, &Checkpoint { timestamp: current_timestamp, votes });
            return;
        }
    }

    // Create new checkpoint
    let key = VotesStorageKey::TotalSupplyCheckpoint(num);
    e.storage().persistent().set(&key, &Checkpoint { timestamp: current_timestamp, votes });

    // Update checkpoint count
    let num_key = VotesStorageKey::NumTotalSupplyCheckpoints;
    e.storage().instance().set(&num_key, &(num + 1));
}
