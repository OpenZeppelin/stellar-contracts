use soroban_sdk::{
    contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env, Map, String, Symbol,
    Val, Vec,
};

use crate::timelock::{
    emit_min_delay_changed, emit_operation_cancelled, emit_operation_executed,
    emit_operation_scheduled, TimelockError, DONE_TIMESTAMP, TIMELOCK_EXTEND_AMOUNT,
    TIMELOCK_TTL_THRESHOLD, UNSET_TIMESTAMP,
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
    /// Minimum delay in seconds for operations
    MinDelay,
    /// Maps operation ID to the timestamp when it will be in a
    /// [`OperationState::Ready`] state (Note: value is 0 for
    /// [`OperationState::Unset`], 1 for [`OperationState:Done`]).
    Timestamp(BytesN<32>),
}

/// Represents a return value from an executed operation.
///
/// This enum wraps all possible Soroban value types that can be returned
/// from contract function invocations. It provides a concrete type that
/// can be properly serialized and displayed by CLI tools.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationResult {
    /// Boolean value
    Bool(bool),
    /// Void/unit type (no return value)
    Void,
    /// 32-bit unsigned integer
    U32(u32),
    /// 32-bit signed integer
    I32(i32),
    /// 64-bit unsigned integer
    U64(u64),
    /// 64-bit signed integer
    I64(i64),
    /// 128-bit unsigned integer
    U128(u128),
    /// 128-bit signed integer
    I128(i128),
    /// 256-bit unsigned integer (as bytes)
    U256(BytesN<32>),
    /// 256-bit signed integer (as bytes)
    I256(BytesN<32>),
    /// Arbitrary bytes
    Bytes(Bytes),
    /// String value
    String(String),
    /// Symbol value
    Symbol(Symbol),
    /// Vector of values
    Vec(Vec<Val>),
    /// Map of key-value pairs
    Map(Map<Val, Val>),
    /// Contract or account address
    Address(Address),
}

// ################## QUERY STATE ##################

/// Converts a `Val` to an `OperationResult`.
///
/// This function attempts to convert a raw `Val` into a concrete
/// `OperationResult` type by checking its underlying type and performing
/// the appropriate conversion.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `val` - The raw value to convert.
///
/// # Returns
///
/// An `OperationResult` containing the converted value. If the value cannot
/// be converted to any supported type, returns `OperationResult::Void`.
///
/// # Note
///
/// The function attempts type conversions in a specific order. For 32-byte
/// values, it cannot distinguish between U256 and I256 without additional
/// context, so all 32-byte values are treated as U256. Contract developers
/// should use more specific return types when possible.
fn val_to_operation_result(e: &Env, val: Val) -> OperationResult {
    use soroban_sdk::TryFromVal;

    // Try each type in order of likelihood/simplicity
    if let Ok(v) = bool::try_from_val(e, &val) {
        return OperationResult::Bool(v);
    }
    if let Ok(v) = u32::try_from_val(e, &val) {
        return OperationResult::U32(v);
    }
    if let Ok(v) = i32::try_from_val(e, &val) {
        return OperationResult::I32(v);
    }
    if let Ok(v) = u64::try_from_val(e, &val) {
        return OperationResult::U64(v);
    }
    if let Ok(v) = i64::try_from_val(e, &val) {
        return OperationResult::I64(v);
    }
    if let Ok(v) = u128::try_from_val(e, &val) {
        return OperationResult::U128(v);
    }
    if let Ok(v) = i128::try_from_val(e, &val) {
        return OperationResult::I128(v);
    }
    if let Ok(v) = Address::try_from_val(e, &val) {
        return OperationResult::Address(v);
    }
    if let Ok(v) = Symbol::try_from_val(e, &val) {
        return OperationResult::Symbol(v);
    }
    if let Ok(v) = String::try_from_val(e, &val) {
        return OperationResult::String(v);
    }
    if let Ok(v) = Bytes::try_from_val(e, &val) {
        // Check if it's a 32-byte value (could be U256 or I256)
        // Note: We cannot distinguish U256 from I256 without additional context
        // from the contract. Both are represented as 32-byte values in Soroban.
        // We default to treating them as U256.
        if v.len() == 32 {
            if let Ok(bytes_n) = BytesN::<32>::try_from_val(e, &val) {
                return OperationResult::U256(bytes_n);
            }
        }
        return OperationResult::Bytes(v);
    }
    if let Ok(v) = Vec::<Val>::try_from_val(e, &val) {
        return OperationResult::Vec(v);
    }
    if let Ok(v) = Map::<Val, Val>::try_from_val(e, &val) {
        return OperationResult::Map(v);
    }

    // If nothing matched, it's likely void
    OperationResult::Void
}

// ################## QUERY STATE ##################

/// Returns the minimum delay in seconds required for operations.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Returns
///
/// The minimum delay in seconds.
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

/// Returns the timestamp at which an operation becomes ready.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
///
/// # Returns
///
/// - `UNSET_TIMESTAMP` for unset operations
/// - `DONE_TIMESTAMP` for done operations
/// - Unix timestamp when the operation becomes ready for scheduled operations
pub fn get_timestamp(e: &Env, operation_id: &BytesN<32>) -> u64 {
    let key = TimelockStorageKey::Timestamp(operation_id.clone());
    if let Some(timestamp) = e.storage().persistent().get::<_, u64>(&key) {
        e.storage().persistent().extend_ttl(&key, TIMELOCK_TTL_THRESHOLD, TIMELOCK_EXTEND_AMOUNT);
        timestamp
    } else {
        UNSET_TIMESTAMP
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
    let ready_timestamp = get_timestamp(e, operation_id);
    let current_timestamp = e.ledger().timestamp();

    match ready_timestamp {
        UNSET_TIMESTAMP => OperationState::Unset,
        DONE_TIMESTAMP => OperationState::Done,
        ready if ready > current_timestamp => OperationState::Waiting,
        _ => OperationState::Ready,
    }
}

/// Returns whether an operation has been scheduled (in any state except Unset).
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation_id` - The unique identifier of the operation.
pub fn operation_exists(e: &Env, operation_id: &BytesN<32>) -> bool {
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

/// Sets the minimum delay required for operations.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `min_delay` - The new minimum delay in seconds.
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
/// * `delay` - The delay in seconds before the operation can be executed.
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

    if operation_exists(e, &id) {
        panic_with_error!(e, TimelockError::OperationAlreadyScheduled);
    }

    // Get minimum delay (will panic if not set)
    let min_delay = get_min_delay(e);

    if delay < min_delay {
        panic_with_error!(e, TimelockError::InsufficientDelay);
    }

    let current_timestamp = e.ledger().timestamp();
    let ready_timestamp = current_timestamp + (delay as u64);

    let key = TimelockStorageKey::Timestamp(id.clone());
    e.storage().persistent().set(&key, &ready_timestamp);

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

/// Executes a scheduled operation by invoking the target contract.
///
/// This is a wrapper around [`set_execute_operation`] that also performs the
/// cross-contract invocation. For self-administration scenarios where the
/// target is the timelock contract itself, use [`set_execute_operation`]
/// directly instead.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation` - The operation to execute.
///
/// # Returns
///
/// The return value from the invoked contract function as an `OperationResult`.
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
pub fn execute_operation(e: &Env, operation: &Operation) -> OperationResult {
    set_execute_operation(e, operation);

    let result =
        e.invoke_contract::<Val>(&operation.target, &operation.function, operation.args.clone());
    val_to_operation_result(e, result)
}

/// Validates and marks an operation as executed without invoking the target.
///
/// This function performs all the validation and state updates for executing
/// an operation, but does not perform the cross-contract invocation. It is
/// used by [`execute_operation`] and can be called directly for
/// self-administration scenarios where the timelock contract needs to execute
/// operations on itself.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operation` - The operation to validate and mark as executed.
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
pub fn set_execute_operation(e: &Env, operation: &Operation) {
    let id = hash_operation(e, operation);

    if !is_operation_ready(e, &id) {
        panic_with_error!(e, TimelockError::InvalidOperationState);
    }

    // Check predecessor is done (if specified)
    let no_predecessor = BytesN::<32>::from_array(e, &[0u8; 32]);
    if operation.predecessor != no_predecessor && !is_operation_done(e, &operation.predecessor) {
        panic_with_error!(e, TimelockError::UnexecutedPredecessor);
    }

    let key = TimelockStorageKey::Timestamp(id.clone());
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
    if !is_operation_pending(e, operation_id) {
        panic_with_error!(e, TimelockError::InvalidOperationState);
    }

    let key = TimelockStorageKey::Timestamp(operation_id.clone());
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
    let mut data = Bytes::new(e);

    data.append(&operation.target.clone().to_xdr(e));
    data.append(&operation.function.clone().to_xdr(e));
    data.append(&operation.args.clone().to_xdr(e));
    data.append(&operation.predecessor.clone().into());
    data.append(&operation.salt.clone().into());

    e.crypto().keccak256(&data).into()
}
