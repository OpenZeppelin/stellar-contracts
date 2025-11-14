//! Timelock Controller Example Contract.
//!
//! This contract demonstrates a complete timelock controller implementation with role-based
//! access control, similar to OpenZeppelin's Solidity TimelockController.
//!
//! # Roles
//!
//! - **Admin**: Can manage all roles and update the minimum delay. By default, the contract
//!   itself is the admin, meaning admin operations must go through the timelock process.
//! - **Proposer**: Can schedule operations. Proposers are also automatically granted the
//!   Canceller role.
//! - **Executor**: Can execute operations that are ready.
//! - **Canceller**: Can cancel pending operations.
//!
//! # Usage
//!
//! 1. Deploy the contract with initial proposers, executors, and minimum delay
//! 2. Proposers schedule operations with a delay >= minimum delay
//! 3. After the delay passes, executors can execute the operations
//! 4. Cancellers can cancel operations before they are executed
//!
//! # Self-Administration
//!
//! The contract is self-administered by default, meaning the contract address itself has the
//! admin role. This ensures that administrative actions (like changing the minimum delay or
//! managing roles) must go through the timelock process, providing transparency and allowing
//! users to react to proposed changes.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol, Val, Vec};
use stellar_access::access_control::{grant_role_no_auth, set_admin, AccessControl};
use stellar_governance::timelock::{cancel_operation, execute_operation, schedule_operation};
use stellar_governance::timelock::{
    get_min_delay as timelock_get_min_delay, get_operation_state, get_timestamp,
    hash_operation as timelock_hash_operation, is_operation, is_operation_done,
    is_operation_pending, is_operation_ready, set_min_delay as timelock_set_min_delay, Operation,
    OperationState,
};
use stellar_macros::{default_impl, only_role};

// Role constants
const PROPOSER_ROLE: Symbol = symbol_short!("proposer");
const EXECUTOR_ROLE: Symbol = symbol_short!("executor");
const CANCELLER_ROLE: Symbol = symbol_short!("canceller");

#[contract]
pub struct TimelockController;

#[contractimpl]
impl TimelockController {
    /// Initializes the timelock controller.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `min_delay` - Initial minimum delay in ledgers for operations.
    /// * `proposers` - Accounts to be granted proposer and canceller roles.
    /// * `executors` - Accounts to be granted executor role.
    /// * `admin` - Optional account to be granted admin role for initial setup.
    ///   If provided, this admin can configure roles without delay but should
    ///   renounce the role after setup to enforce timelock governance.
    ///
    /// # Notes
    ///
    /// - The contract itself is always granted the admin role for self-administration.
    /// - Proposers are automatically granted the canceller role.
    /// - If an external admin is provided, they should renounce their admin role after
    ///   initial configuration to ensure all admin actions go through the timelock.
    pub fn __constructor(
        e: &Env,
        min_delay: u32,
        proposers: Vec<Address>,
        executors: Vec<Address>,
        admin: Option<Address>,
    ) {
        let admin_addr = match admin {
            Some(admin_addr) => admin_addr,
            _ => e.current_contract_address(),
        };
        set_admin(e, &admin_addr);

        // Register proposers and cancellers
        for proposer in proposers.iter() {
            grant_role_no_auth(e, &admin_addr, &proposer, &PROPOSER_ROLE);
            grant_role_no_auth(e, &admin_addr, &proposer, &CANCELLER_ROLE);
        }

        // Register executors
        for executor in executors.iter() {
            grant_role_no_auth(e, &admin_addr, &executor, &EXECUTOR_ROLE);
        }

        // Set minimum delay
        timelock_set_min_delay(e, min_delay);
    }

    /// Schedules an operation for execution after a delay.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `target` - The target contract address.
    /// * `function` - The function name to invoke.
    /// * `args` - The arguments to pass to the function.
    /// * `predecessor` - The predecessor operation ID (use empty bytes for none).
    /// * `salt` - Salt for uniqueness (use empty bytes for default).
    /// * `delay` - The delay in ledgers before the operation can be executed.
    /// * `proposer` - The address proposing the operation (must have proposer role).
    ///
    /// # Returns
    ///
    /// The unique identifier (hash) of the scheduled operation.
    ///
    /// # Notes
    ///
    /// * Authorization for `proposer` is required.
    /// * The proposer must have the PROPOSER_ROLE.
    #[allow(clippy::too_many_arguments)]
    #[only_role(proposer, "proposer")]
    pub fn schedule_op(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
        delay: u32,
        proposer: Address,
    ) -> BytesN<32> {
        let operation = Operation { target, function, args, predecessor, salt };
        schedule_operation(e, &operation, delay)
    }

    /// Executes a scheduled operation that is ready.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `target` - The target contract address.
    /// * `function` - The function name to invoke.
    /// * `args` - The arguments to pass to the function.
    /// * `predecessor` - The predecessor operation ID.
    /// * `salt` - Salt for uniqueness.
    /// * `executor` - The address executing the operation (must have executor role).
    ///
    /// # Returns
    ///
    /// The return value from the executed operation.
    ///
    /// # Notes
    ///
    /// * Authorization for `executor` is required.
    /// * The executor must have the EXECUTOR_ROLE.
    #[only_role(executor, "executor")]
    pub fn execute_op(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
        executor: Address,
    ) -> Val {
        let operation = Operation { target, function, args, predecessor, salt };
        execute_operation(e, &operation)
    }

    /// Cancels a scheduled operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `operation_id` - The unique identifier of the operation to cancel.
    /// * `canceller` - The address cancelling the operation (must have canceller role).
    ///
    /// # Notes
    ///
    /// * Authorization for `canceller` is required.
    /// * The canceller must have the CANCELLER_ROLE.
    #[only_role(canceller, "canceller")]
    pub fn cancel_op(e: &Env, operation_id: BytesN<32>, canceller: Address) {
        cancel_operation(e, &operation_id);
    }

    /// Updates the minimum delay for future operations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_delay` - The new minimum delay in ledgers.
    ///
    /// # Notes
    ///
    /// * Authorization for `admin` is required.
    /// * This function should typically be called through the timelock itself
    ///   (self-administration) to ensure transparency.
    pub fn update_delay(e: &Env, new_delay: u32) {
        // TODO: what to do here?
        e.current_contract_address().require_auth();
        timelock_set_min_delay(e, new_delay);
    }

    /// Returns the minimum delay in ledgers required for operations.
    pub fn get_min_delay(e: &Env) -> u32 {
        timelock_get_min_delay(e)
    }

    /// Computes the unique identifier for an operation.
    pub fn hash_operation(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
    ) -> BytesN<32> {
        let operation = Operation { target, function, args, predecessor, salt };
        timelock_hash_operation(e, &operation)
    }

    /// Returns the ledger number at which an operation becomes ready.
    pub fn get_timestamp(e: &Env, operation_id: BytesN<32>) -> u32 {
        get_timestamp(e, &operation_id)
    }

    /// Returns the current state of an operation.
    pub fn get_operation_state(e: &Env, operation_id: BytesN<32>) -> OperationState {
        get_operation_state(e, &operation_id)
    }

    /// Returns whether an operation exists (scheduled or done).
    pub fn is_operation(e: &Env, operation_id: BytesN<32>) -> bool {
        is_operation(e, &operation_id)
    }

    /// Returns whether an operation is pending (waiting or ready).
    pub fn is_operation_pending(e: &Env, operation_id: BytesN<32>) -> bool {
        is_operation_pending(e, &operation_id)
    }

    /// Returns whether an operation is ready for execution.
    pub fn is_operation_ready(e: &Env, operation_id: BytesN<32>) -> bool {
        is_operation_ready(e, &operation_id)
    }

    /// Returns whether an operation has been executed.
    pub fn is_operation_done(e: &Env, operation_id: BytesN<32>) -> bool {
        is_operation_done(e, &operation_id)
    }
}

// Implement AccessControl trait to expose role management functions
#[default_impl]
#[contractimpl]
impl AccessControl for TimelockController {}
