//! # Timelock Module
//!
//! This module provides functionality for time-delayed execution of operations,
//! enabling governance mechanisms where actions must wait for a minimum delay
//! before execution.
//!
//! The timelock enforces a delay between scheduling an operation and executing
//! it, giving stakeholders time to review and potentially cancel dangerous
//! operations.
//!
//! # Core Concepts
//!
//! - **Operations**: Actions to be executed on target contracts
//! - **Scheduling**: Proposing an operation with a delay period
//! - **Execution**: Running the operation after the delay has passed
//! - **Cancellation**: Removing a scheduled operation before execution
//! - **Predecessors**: Dependencies between operations (operation B requires
//!   operation A to be done first)
//!
//! # Usage
//!
//! This module provides storage functions that can be integrated into a
//! contract. The contract is responsible for:
//! - Authorization checks (who can schedule/execute/cancel)
//! - Initialization of minimum delay
//!
//! # Example
//!
//! ```ignore
//! use stellar_governance::timelock::{
//!     schedule_operation, execute_operation, get_operation_state, OperationState
//! };
//!
//! // In your contract:
//! pub fn schedule(e: &Env, operation: Operation, delay: u32) {
//!     // Add authorization checks here
//!     let id = schedule_operation(e, &operation, delay);
//!     // Emit events
//! }
//!
//! pub fn execute(e: &Env, operation: Operation) {
//!     // Add authorization checks here
//!     execute_operation(e, &operation);
//!     // Emit events
//! }
//! ```

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, Address, BytesN, Env, Symbol, Val, Vec};

pub use crate::timelock::storage::{
    cancel_operation, execute_operation, get_min_delay, get_operation_state, get_timestamp,
    hash_operation, is_operation, is_operation_done, is_operation_pending, is_operation_ready,
    schedule_operation, set_execute_operation, set_min_delay, Operation, OperationState,
    TimelockStorageKey,
};

// ################## ERRORS ##################

/// Errors that can occur in timelock operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TimelockError {
    /// The operation is already scheduled
    OperationAlreadyScheduled = 4000,
    /// The delay is less than the minimum required delay
    InsufficientDelay = 4001,
    /// The operation is not in the expected state
    InvalidOperationState = 4002,
    /// A predecessor operation has not been executed yet
    UnexecutedPredecessor = 4003,
    /// The caller is not authorized to perform this action
    Unauthorized = 4004,
    /// The minimum delay has not been set
    MinDelayNotSet = 4005,
    /// The operation has not been scheduled
    OperationNotScheduled = 4006,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL threshold for extending storage entries (in ledgers)
pub const TIMELOCK_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL extension amount for storage entries (in ledgers)
pub const TIMELOCK_TTL_THRESHOLD: u32 = TIMELOCK_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Sentinel value for an operation that has not been scheduled.
pub const UNSET_TIMESTAMP: u32 = 0;

/// Sentinel value used to mark an operation as done.
/// Using 1 instead of 0 to distinguish from unset operations.
pub const DONE_TIMESTAMP: u32 = 1;

// ################## EVENTS ##################

/// Event emitted when the minimum delay is changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinDelayChanged {
    pub old_delay: u32,
    pub new_delay: u32,
}

/// Emits an event when the minimum delay is changed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `old_delay` - The previous minimum delay value.
/// * `new_delay` - The new minimum delay value.
pub fn emit_min_delay_changed(e: &Env, old_delay: u32, new_delay: u32) {
    MinDelayChanged { old_delay, new_delay }.publish(e);
}

/// Event emitted when an operation is scheduled.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationScheduled {
    #[topic]
    pub id: BytesN<32>,
    #[topic]
    pub target: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub predecessor: BytesN<32>,
    pub salt: BytesN<32>,
    pub delay: u32,
}

/// Emits an event when an operation is scheduled.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `id` - The unique identifier of the operation.
/// * `target` - The target contract address.
/// * `function` - The function name to invoke.
/// * `args` - The arguments to pass to the function.
/// * `predecessor` - The predecessor operation ID.
/// * `salt` - The salt for uniqueness.
/// * `delay` - The delay in ledgers.
#[allow(clippy::too_many_arguments)]
pub fn emit_operation_scheduled(
    e: &Env,
    id: &BytesN<32>,
    target: &Address,
    function: &Symbol,
    args: &Vec<Val>,
    predecessor: &BytesN<32>,
    salt: &BytesN<32>,
    delay: u32,
) {
    OperationScheduled {
        id: id.clone(),
        target: target.clone(),
        function: function.clone(),
        args: args.clone(),
        predecessor: predecessor.clone(),
        salt: salt.clone(),
        delay,
    }
    .publish(e);
}

/// Event emitted when an operation is executed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationExecuted {
    #[topic]
    pub id: BytesN<32>,
    #[topic]
    pub target: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub predecessor: BytesN<32>,
    pub salt: BytesN<32>,
}

/// Emits an event when an operation is executed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `id` - The unique identifier of the operation.
/// * `target` - The target contract address.
/// * `function` - The function name to invoke.
/// * `args` - The arguments to pass to the function.
/// * `predecessor` - The predecessor operation ID.
/// * `salt` - The salt for uniqueness.
pub fn emit_operation_executed(
    e: &Env,
    id: &BytesN<32>,
    target: &Address,
    function: &Symbol,
    args: &Vec<Val>,
    predecessor: &BytesN<32>,
    salt: &BytesN<32>,
) {
    OperationExecuted {
        id: id.clone(),
        target: target.clone(),
        function: function.clone(),
        args: args.clone(),
        predecessor: predecessor.clone(),
        salt: salt.clone(),
    }
    .publish(e);
}

/// Event emitted when an operation is cancelled.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCancelled {
    #[topic]
    pub id: BytesN<32>,
}

/// Emits an event when an operation is cancelled.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `id` - The unique identifier of the operation.
pub fn emit_operation_cancelled(e: &Env, id: &BytesN<32>) {
    OperationCancelled { id: id.clone() }.publish(e);
}
