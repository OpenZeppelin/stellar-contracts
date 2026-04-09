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
//! Implement the [`Timelock`] trait on a contract to expose a standard
//! timelock interface. The trait generates a `TimelockClient` that other
//! contracts can use for type-safe cross-contract calls.
//!
//! The contract is responsible for:
//! - Authorization checks (who can schedule/execute/cancel)
//! - Initialization of minimum delay
//!
//! # Example
//!
//! ```ignore
//! use stellar_governance::timelock::{
//!     Timelock, schedule_operation, execute_operation, Operation,
//! };
//!
//! #[contractimpl(contracttrait)]
//! impl Timelock for MyTimelockController {
//!     fn schedule_op(e: &Env, target: Address, ...) -> BytesN<32> {
//!         // Add authorization checks here
//!         let operation = Operation { target, function, args, predecessor, salt };
//!         schedule_operation(e, &operation, delay)
//!     }
//!
//!     fn execute_op(e: &Env, target: Address, ...) -> Val {
//!         // Add authorization checks here
//!         let operation = Operation { target, function, args, predecessor, salt };
//!         execute_operation(e, &operation)
//!     }
//!     // ...
//! }
//! ```

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, Address, BytesN, Env, Symbol, Val, Vec,
};

pub use crate::timelock::storage::{
    cancel_operation, execute_operation, get_min_delay, get_operation_ledger, get_operation_state,
    hash_operation, is_operation_done, is_operation_pending, is_operation_ready, operation_exists,
    schedule_operation, set_execute_operation, set_min_delay, Operation, OperationState,
    TimelockStorageKey,
};

/// Trait for timelock controller contracts.
///
/// The `Timelock` trait defines the interface for time-delayed execution of
/// operations, enabling governance mechanisms where actions must wait for a
/// minimum delay before execution. Implementing this trait generates a
/// `TimelockClient` that other contracts can use for type-safe cross-contract
/// calls.
///
/// # Methods Without Default Implementation
///
/// The following methods have no default implementation because they require
/// access-control logic that varies per contract:
///
/// - [`Timelock::schedule_op`]
/// - [`Timelock::execute_op`]
/// - [`Timelock::cancel_op`]
/// - [`Timelock::update_delay`]
#[contracttrait]
pub trait Timelock {
    /// Returns the minimum delay in ledgers required for operations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`TimelockError::MinDelayNotSet`] - If the minimum delay has not been
    ///   initialized.
    fn get_min_delay(e: &Env) -> u32 {
        storage::get_min_delay(e)
    }

    /// Computes the unique identifier for an operation from its parameters.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `target` - The target contract address.
    /// * `function` - The function name to invoke.
    /// * `args` - The arguments to pass to the function.
    /// * `predecessor` - The predecessor operation ID.
    /// * `salt` - The salt for uniqueness.
    fn hash_operation(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
    ) -> BytesN<32> {
        let operation = Operation { target, function, args, predecessor, salt };
        storage::hash_operation(e, &operation)
    }

    /// Returns the ledger sequence number at which an operation becomes ready.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation.
    fn get_operation_ledger(e: &Env, operation_id: BytesN<32>) -> u32 {
        storage::get_operation_ledger(e, &operation_id)
    }

    /// Returns the current state of an operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation.
    fn get_operation_state(e: &Env, operation_id: BytesN<32>) -> OperationState {
        storage::get_operation_state(e, &operation_id)
    }

    /// Returns whether an operation exists (scheduled or done).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation.
    fn operation_exists(e: &Env, operation_id: BytesN<32>) -> bool {
        storage::operation_exists(e, &operation_id)
    }

    /// Returns whether an operation is pending (waiting or ready).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation.
    fn is_operation_pending(e: &Env, operation_id: BytesN<32>) -> bool {
        storage::is_operation_pending(e, &operation_id)
    }

    /// Returns whether an operation is ready for execution.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation.
    fn is_operation_ready(e: &Env, operation_id: BytesN<32>) -> bool {
        storage::is_operation_ready(e, &operation_id)
    }

    /// Returns whether an operation has been executed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation.
    fn is_operation_done(e: &Env, operation_id: BytesN<32>) -> bool {
        storage::is_operation_done(e, &operation_id)
    }

    /// Schedules an operation for execution after a delay and returns the
    /// unique operation identifier.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `target` - The target contract address.
    /// * `function` - The function name to invoke on the target.
    /// * `args` - The arguments to pass to the function.
    /// * `predecessor` - The predecessor operation ID. Use
    ///   `BytesN::<32>::from_array(&[0u8; 32])` for no predecessor.
    /// * `salt` - A salt value for uniqueness. Allows scheduling the same
    ///   operation multiple times with different IDs.
    /// * `delay` - The delay in ledgers before the operation can be executed.
    /// * `proposer` - The address proposing the operation.
    ///
    /// # Errors
    ///
    /// * [`TimelockError::OperationAlreadyScheduled`] - If an operation with
    ///   the same ID is already scheduled.
    /// * [`TimelockError::InsufficientDelay`] - If `delay` is less than the
    ///   minimum required delay.
    /// * [`TimelockError::MinDelayNotSet`] - If the minimum delay has not been
    ///   initialized.
    ///
    /// # Events
    ///
    /// * topics - `["operation_scheduled", id: BytesN<32>, target: Address]`
    /// * data - `[function: Symbol, args: Vec<Val>, predecessor: BytesN<32>,
    ///   salt: BytesN<32>, delay: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for `proposer` is required.
    /// * The implementer must verify that `proposer` has the appropriate role,
    ///   construct an [`Operation`], and call [`schedule_operation`].
    #[allow(clippy::too_many_arguments)]
    fn schedule_op(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
        delay: u32,
        proposer: Address,
    ) -> BytesN<32>;

    /// Executes a scheduled operation that is ready and returns the result of
    /// the target contract invocation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `target` - The target contract address.
    /// * `function` - The function name to invoke on the target.
    /// * `args` - The arguments to pass to the function.
    /// * `predecessor` - The predecessor operation ID.
    /// * `salt` - The salt value used when scheduling.
    /// * `executor` - The address executing the operation, or `None` if open
    ///   execution is allowed.
    ///
    /// # Errors
    ///
    /// * [`TimelockError::InvalidOperationState`] - If the operation is not in
    ///   the `Ready` state.
    /// * [`TimelockError::UnexecutedPredecessor`] - If the predecessor
    ///   operation has not been executed.
    ///
    /// # Events
    ///
    /// * topics - `["operation_executed", id: BytesN<32>, target: Address]`
    /// * data - `[function: Symbol, args: Vec<Val>, predecessor: BytesN<32>,
    ///   salt: BytesN<32>]`
    ///
    /// # Notes
    ///
    /// * Authorization for `executor` is optional (open execution is allowed
    ///   when `executor` is `None`).
    /// * The implementer must construct an [`Operation`] and call
    ///   [`execute_operation`].
    fn execute_op(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
        executor: Option<Address>,
    ) -> Val;

    /// Cancels a pending operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operation_id` - The unique identifier of the operation to cancel.
    /// * `canceller` - The address cancelling the operation.
    ///
    /// # Errors
    ///
    /// * [`TimelockError::OperationNotScheduled`] - If the operation has not
    ///   been scheduled.
    /// * [`TimelockError::InvalidOperationState`] - If the operation is not in
    ///   a pending state.
    ///
    /// # Events
    ///
    /// * topics - `["operation_cancelled", id: BytesN<32>]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// * Authorization for `canceller` is required.
    /// * The implementer must verify that `canceller` has the appropriate role
    ///   and call [`cancel_operation`].
    fn cancel_op(e: &Env, operation_id: BytesN<32>, canceller: Address);

    /// Updates the minimum delay for future operations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `new_delay` - The new minimum delay in ledgers.
    /// * `operator` - The address updating the delay.
    ///
    /// # Events
    ///
    /// * topics - `["min_delay_changed"]`
    /// * data - `[old_delay: u32, new_delay: u32]`
    ///
    /// # Notes
    ///
    /// * The implementer must verify that `operator` has administrative
    ///   privileges and call [`set_min_delay`].
    fn update_delay(e: &Env, new_delay: u32, operator: Address);
}

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
pub const UNSET_LEDGER: u32 = 0;

/// Sentinel value used to mark an operation as done.
/// Using 1 instead of 0 to distinguish from unset operations.
pub const DONE_LEDGER: u32 = 1;

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
