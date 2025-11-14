use soroban_sdk::{
    contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env, Symbol, Val, Vec,
};

use crate::timelock::{
    emit_min_delay_changed, emit_operation_cancelled, emit_operation_executed,
    emit_operation_scheduled, TimelockError, DONE_TIMESTAMP, TIMELOCK_EXTEND_AMOUNT,
    TIMELOCK_TTL_THRESHOLD,
};

// ################## TYPES ##################

/// Represents a operation to be executed by the timelock.
///
/// An operation encapsulates all the information needed to invoke a function
/// on a target contract after the timelock delay has passed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    /// The contract address to call
    pub target: Address,
    /// The function name to invoke on the target contract
    pub function: Symbol,
    /// The serialized arguments to pass to the function
    pub args: Vec<Val>,
    /// Hash of a predecessor operation that must be executed first.
    /// Use BytesN::<32>::from_array(&[0u8; 32]) for no predecessor.
    pub predecessor: BytesN<32>,
    /// A salt value for operation uniqueness.
    /// Allows scheduling the same operation multiple times with different IDs.
    pub salt: BytesN<32>,
}

/// The state of an operation in the timelock system.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum OperationState {
    /// Operation has not been scheduled
    Unset,
    /// Operation is scheduled but the delay period has not passed
    Waiting,
    /// Operation is ready to be executed (delay has passed)
    Ready,
    /// Operation has been executed
    Done,
}

/// Storage keys for the timelock module.
#[derive(Clone)]
#[contracttype]
pub enum TimelockStorageKey {
    /// Minimum delay in ledgers for operations
    MinDelay,
    /// Maps operation ID to the ledger number when it becomes ready
    /// Value is 0 for unset, 1 for done, or ledger number for scheduled
    Timestamps(BytesN<32>),
}

// ################## QUERY STATE ##################

/// Returns the minimum delay in ledgers required for operations.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Returns
///
/// The minimum delay in ledgers.
///
/// # Errors
///
/// * [`TimelockError::MinDelayNotSet`] - If the minimum delay has not been set.
pub fn get_min_delay(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&TimelockStorageKey::MinDelay)
        .unwrap_or_else(|| panic_with_error!(e, TimelockError::MinDelayNotSet))
}

/// Returns the ledger number at which an operation becomes ready.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
///
/// # Returns
///
/// - `0` for unset operations
/// - `1` for done operations
/// - Ledger number when the operation becomes ready for scheduled operations
pub fn get_timestamp(e: &Env, operation_id: &BytesN<32>) -> u32 {
    let key = TimelockStorageKey::Timestamps(operation_id.clone());
    if let Some(timestamp) = e.storage().persistent().get::<_, u32>(&key) {
        e.storage().persistent().extend_ttl(&key, TIMELOCK_TTL_THRESHOLD, TIMELOCK_EXTEND_AMOUNT);
        timestamp
    } else {
        0
    }
}

/// Returns the current state of an operation.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
///
/// # Returns
///
/// The current [`OperationState`] of the operation.
pub fn get_operation_state(e: &Env, operation_id: &BytesN<32>) -> OperationState {
    let timestamp = get_timestamp(e, operation_id);
    let current_ledger = e.ledger().sequence();

    match timestamp {
        0 => OperationState::Unset,
        DONE_TIMESTAMP => OperationState::Done,
        ready_ledger if ready_ledger > current_ledger => OperationState::Waiting,
        _ => OperationState::Ready,
    }
}

/// Returns whether an operation has been scheduled (in any state except Unset).
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
pub fn is_operation(e: &Env, operation_id: &BytesN<32>) -> bool {
    get_operation_state(e, operation_id) != OperationState::Unset
}

/// Returns whether an operation is pending (Waiting or Ready).
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
pub fn is_operation_pending(e: &Env, operation_id: &BytesN<32>) -> bool {
    let state = get_operation_state(e, operation_id);
    state == OperationState::Waiting || state == OperationState::Ready
}

/// Returns whether an operation is ready for execution.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
pub fn is_operation_ready(e: &Env, operation_id: &BytesN<32>) -> bool {
    get_operation_state(e, operation_id) == OperationState::Ready
}

/// Returns whether an operation has been executed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
pub fn is_operation_done(e: &Env, operation_id: &BytesN<32>) -> bool {
    get_operation_state(e, operation_id) == OperationState::Done
}

// ################## CHANGE STATE ##################

/// Sets the minimum delay for future operations.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `min_delay` - The minimum delay in ledgers.
///
/// # Events
///
/// * topics - `["min_delay_changed"]`
/// * data - `[old_delay: u32, new_delay: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn set_min_delay(e: &Env, min_delay: u32) {
    let old_delay =
        e.storage().instance().get::<_, u32>(&TimelockStorageKey::MinDelay).unwrap_or(0);
    e.storage().instance().set(&TimelockStorageKey::MinDelay, &min_delay);
    emit_min_delay_changed(e, old_delay, min_delay);
}

/// Schedules an operation for execution after a delay.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation` - The operation to schedule.
/// * `delay` - The delay in ledgers before the operation can be executed.
///
/// # Returns
///
/// The unique identifier (hash) of the scheduled operation.
///
/// # Errors
///
/// * [`TimelockError::OperationAlreadyScheduled`] - If the operation is already
///   scheduled.
/// * [`TimelockError::InsufficientDelay`] - If the delay is less than the
///   minimum delay.
/// * [`TimelockError::MinDelayNotSet`] - If the minimum delay has not been
///   initialized.
///
/// # Events
///
/// * topics - `["operation_scheduled", id: BytesN<32>, target: Address]`
/// * data - `[function: Symbol, args: Vec<Val>, predecessor: BytesN<32>, salt:
///   BytesN<32>, delay: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn schedule_operation(e: &Env, operation: &Operation, delay: u32) -> BytesN<32> {
    let id = hash_operation(e, operation);

    // Check if operation is already scheduled
    if is_operation(e, &id) {
        panic_with_error!(e, TimelockError::OperationAlreadyScheduled);
    }

    // Get minimum delay (will panic if not set)
    let min_delay = get_min_delay(e);

    // Check delay is sufficient
    if delay < min_delay {
        panic_with_error!(e, TimelockError::InsufficientDelay);
    }

    // Calculate ready ledger
    let current_ledger = e.ledger().sequence();
    let ready_ledger = current_ledger + delay;

    // Store the timestamp
    let key = TimelockStorageKey::Timestamps(id.clone());
    e.storage().persistent().set(&key, &ready_ledger);

    emit_operation_scheduled(
        e,
        &id,
        &operation.target,
        &operation.function,
        &operation.args,
        &operation.predecessor,
        &operation.salt,
        delay,
    );

    id
}

/// Executes a scheduled operation.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation` - The operation to execute.
///
/// # Errors
///
/// * [`TimelockError::InvalidOperationState`] - If the operation is not ready
///   for execution.
/// * [`TimelockError::UnexecutedPredecessor`] - If the predecessor operation
///   has not been executed.
///
/// # Events
///
/// * topics - `["operation_executed", id: BytesN<32>, target: Address]`
/// * data - `[function: Symbol, args: Vec<Val>, predecessor: BytesN<32>, salt:
///   BytesN<32>]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn execute_operation(e: &Env, operation: &Operation) -> Val {
    set_execute_operation(e, operation);

    e.invoke_contract::<Val>(&operation.target, &operation.function, operation.args.clone())
}

pub fn set_execute_operation(e: &Env, operation: &Operation) -> Val {
    let id = hash_operation(e, operation);

    // Check operation is ready
    if !is_operation_ready(e, &id) {
        panic_with_error!(e, TimelockError::InvalidOperationState);
    }

    // Check predecessor is done (if specified)
    let no_predecessor = BytesN::<32>::from_array(e, &[0u8; 32]);
    if operation.predecessor != no_predecessor && !is_operation_done(e, &operation.predecessor) {
        panic_with_error!(e, TimelockError::UnexecutedPredecessor);
    }

    // Mark as done
    let key = TimelockStorageKey::Timestamps(id.clone());
    e.storage().persistent().set(&key, &DONE_TIMESTAMP);

    emit_operation_executed(
        e,
        &id,
        &operation.target,
        &operation.function,
        &operation.args,
        &operation.predecessor,
        &operation.salt,
    );
}

/// Cancels a scheduled operation.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation to cancel.
///
/// # Errors
///
/// * [`TimelockError::InvalidOperationState`] - If the operation is not pending
///   (must be Waiting or Ready).
///
/// # Events
///
/// * topics - `["operation_cancelled", id: BytesN<32>]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn cancel_operation(e: &Env, operation_id: &BytesN<32>) {
    // Check operation is pending
    if !is_operation_pending(e, operation_id) {
        panic_with_error!(e, TimelockError::InvalidOperationState);
    }

    // Remove the timestamp
    let key = TimelockStorageKey::Timestamps(operation_id.clone());
    e.storage().persistent().remove(&key);

    emit_operation_cancelled(e, operation_id);
}

// ################## HASHING ##################

/// Computes the unique identifier for a operation.
///
/// The operation ID is derived from all operation parameters using Keccak256.
/// This ensures that the same operation parameters always produce the same ID,
/// unless the salt is changed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation` - The operation to hash.
///
/// # Returns
///
/// A 32-byte hash uniquely identifying the operation.
pub fn hash_operation(e: &Env, operation: &Operation) -> BytesN<32> {
    // Serialize all operation components
    let mut data = Bytes::new(e);

    // Append target address
    data.append(&operation.target.clone().to_xdr(e));

    // Append function symbol
    data.append(&operation.function.clone().to_xdr(e));

    // Append args
    data.append(&operation.args.clone().to_xdr(e));

    // Append predecessor
    data.append(&operation.predecessor.clone().into());

    // Append salt
    data.append(&operation.salt.clone().into());

    // Hash the concatenated data
    e.crypto().keccak256(&data).into()
}
