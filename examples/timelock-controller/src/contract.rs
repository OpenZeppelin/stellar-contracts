//! Timelock Controller Example Contract.
//!
//! This contract demonstrates a complete timelock controller implementation
//! with role-based access control.
//!
//! # Architecture
//!
//! ```text
//!                ┌─────────┐
//!                │  Admin  │
//!                └────┬────┘
//!                     │
//!          ┌──────────┼──────────┐
//!          │          │          │
//!          │    update_delay()   │
//!          │          │          │
//!     ┌────▼───┐      │     ┌────▼────┐
//!     │Proposer│      │     │Executor │
//!     └────┬───┘      │     └────┬────┘
//!          │          │          │
//! schedule_op()       │    execute_op()
//! cancel_op()         │          │
//!          │          │          │
//!          │          ▼          │
//!          │   ┌─────────────┐   │
//!          └──►│  Timelock   │◄──┘
//!              │ Controller  │◄─── (self-admin when Admin == contract)
//!              └──────┬──────┘
//!                     │
//!              invoke (as owner)
//!                     │
//!                     ▼
//!              ┌─────────────┐
//!              │   Target    │
//!              │  Contract   │
//!              └─────────────┘
//! ```
//!
//! # Roles
//!
//! - **Admin**: Can manage all roles and update the minimum delay. By default,
//!   the contract itself is the admin, meaning admin operations must go through
//!   the timelock process.
//! - **Proposer**: Can schedule operations. Proposers are also automatically
//!   granted the Canceller role.
//! - **Executor**: Can execute operations that are ready. If no executors are
//!   configured, anyone can execute ready operations.
//! - **Canceller**: Can cancel pending operations.
//!
//! # Usage Pattern
//!
//! The timelock controller is typically set as the **owner** of target
//! contracts. This ensures that all privileged operations on those
//! contracts must go through the timelock's proposal lifecycle, providing
//! transparency and allowing time for review before execution.
//!
//! ## Operations on Target Contracts
//!
//! When the timelock controller "owns" a target contract, the proposal
//! lifecycle is:
//!
//! 1. Proposer schedules operations targeting owner-protected functions on the
//!    target contract with a delay >= minimum delay
//! 2. The delay period allows stakeholders to review the proposed changes
//! 3. After the delay passes, executor (or anyone if no executors are
//!    configured) calls `execute_op` to invoke the target contract function
//! 4. Canceller can cancel pending operations before execution
//!
//! **Example**: If a token contract has `mint()` protected by owner
//! authorization, and the timelock is the owner, then minting new tokens
//! requires scheduling through the timelock, waiting for the delay, and then
//! executing.
//!
//! ## Self-Administration Operations
//!
//! When the contract is deployed with `admin` set to `None`, the contract
//! address itself becomes the admin (self-administration). For
//! self-administration operations (e.g., updating the minimum delay, granting
//! and revoking roles), the proposal lifecycle is:
//!
//! 1. Proposer schedules the operation targeting the timelock contract itself
//! 2. After the delay passes, call the admin function directly (not via
//!    `execute_op`)
//! 3. The `CustomAccountInterface` implementation validates the operation is
//!    ready and marks it as executed by checking the executor's role and
//!    authorization
//!
//! This approach ensures administrative changes go through the timelock
//! process.
//!
//! **Note**: Self-administration requires special handling because Soroban does
//! not allow re-entrancy: a contract cannot call its own public functions
//! during execution (e.g., `execute_op` cannot internally call `update_delay`
//! on the same contract). To work around this, the `CustomAccountInterface`
//! implementation validates and marks operations as executed without performing
//! the cross-contract call, allowing admin functions to be called directly.
//!
//! ## Optional External Admin
//!
//! An optional external admin can be provided during deployment to aid with
//! initial configuration of roles after deployment without being subject to
//! delay. However, this role should be subsequently renounced in favor of
//! administration through timelocked proposals to ensure all administrative
//! actions have proper oversight and transparency.

use soroban_sdk::{
    auth::{Context, ContractContext, CustomAccountInterface},
    contract, contractimpl, contracttype,
    crypto::Hash,
    panic_with_error, symbol_short, Address, BytesN, Env, IntoVal, Symbol, Val, Vec,
};
use stellar_access::access_control::{
    ensure_role, get_role_member_count, grant_role_no_auth, set_admin, AccessControl,
};
use stellar_governance::timelock::{
    cancel_operation, execute_operation, get_min_delay as timelock_get_min_delay,
    get_operation_state, get_timestamp, hash_operation as timelock_hash_operation,
    is_operation_done, is_operation_pending, is_operation_ready, operation_exists,
    schedule_operation, set_execute_operation, set_min_delay as timelock_set_min_delay, Operation,
    OperationState, TimelockError,
};
use stellar_macros::{default_impl, only_admin, only_role};

// Role constants
const PROPOSER_ROLE: Symbol = symbol_short!("proposer");
const EXECUTOR_ROLE: Symbol = symbol_short!("executor");
const CANCELLER_ROLE: Symbol = symbol_short!("canceller");

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct OperationMeta {
    pub predecessor: BytesN<32>,
    pub salt: BytesN<32>,
    pub executor: Option<Address>,
}

#[contract]
pub struct TimelockController;

#[contractimpl]
impl CustomAccountInterface for TimelockController {
    type Error = TimelockError;
    type Signature = Vec<OperationMeta>;

    /// Custom authorization check for self-administration operations.
    ///
    /// This enables the timelock contract to execute operations on itself when
    /// the admin is set to the contract's own address. Unlike external
    /// operations which use `execute_op`, self-administration operations are
    /// executed by calling the admin function directly (e.g., `update_delay`,
    /// `grant_role`).
    ///
    /// The `__check_auth` implementation validates that:
    /// - The operation targets the timelock contract itself
    /// - The operation was properly scheduled and is ready for execution
    /// - The predecessor and salt match the scheduled operation
    /// - The executor (if any) has role and has authorized the invocation
    ///
    /// The caller must construct an `OperationMeta` signature containing the
    /// `predecessor` and `salt` values that were used when scheduling the
    /// operation, allowing this function to validate and mark the operation as
    /// executed.
    fn __check_auth(
        e: Env,
        _signature_payload: Hash<32>,
        context_meta: Vec<OperationMeta>,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        for (context, meta) in auth_contexts.iter().zip(context_meta) {
            match context.clone() {
                Context::Contract(ContractContext { contract, fn_name, args }) => {
                    // Allow only for self-administration
                    if contract != e.current_contract_address() {
                        panic_with_error!(&e, TimelockError::Unauthorized)
                    }

                    // If no accounts have EXECUTOR_ROLE, anyone can execute a ready operation
                    if get_role_member_count(&e, &EXECUTOR_ROLE) != 0 {
                        // Check the role and the authorization of the executor
                        let args_for_auth = (
                            // adding an additional symbol argument so that intention for the
                            // authorizer is more explicit
                            Symbol::new(&e, "execute_op"),
                            contract.clone(),
                            fn_name.clone(),
                            args.clone(),
                            meta.predecessor.clone(),
                            meta.salt.clone(),
                        )
                            .into_val(&e);

                        let executor = meta.executor.expect("Executor must be present");

                        ensure_role(&e, &EXECUTOR_ROLE, &executor);
                        executor.require_auth_for_args(args_for_auth);
                    }

                    let op = Operation {
                        target: contract,
                        function: fn_name,
                        args,
                        predecessor: meta.predecessor,
                        salt: meta.salt,
                    };
                    set_execute_operation(&e, &op);
                }
                _ => panic_with_error!(&e, TimelockError::Unauthorized),
            }
        }
        Ok(())
    }
}

#[contractimpl]
impl TimelockController {
    /// Initializes the timelock controller.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `min_delay` - Initial minimum delay in seconds for operations.
    /// * `proposers` - Accounts to be granted proposer and canceller roles.
    /// * `executors` - Accounts to be granted executor role.
    /// * `admin` - Optional account to be granted admin role for initial setup.
    ///   If provided, this admin can configure roles without delay but should
    ///   renounce the role after setup to enforce timelock governance.
    ///
    /// # Notes
    ///
    /// - The contract itself is always granted the admin role for
    ///   self-administration.
    /// - Proposers are automatically granted the canceller role.
    /// - If an external admin is provided, they should renounce their admin
    ///   role after initial configuration to ensure all admin actions go
    ///   through the timelock.
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

        // Grant to all initial proposers a canceller role
        for proposer in proposers.iter() {
            grant_role_no_auth(e, &proposer, &PROPOSER_ROLE, &admin_addr);
            grant_role_no_auth(e, &proposer, &CANCELLER_ROLE, &admin_addr);
        }

        for executor in executors.iter() {
            grant_role_no_auth(e, &executor, &EXECUTOR_ROLE, &admin_addr);
        }

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
    /// * `predecessor` - The predecessor operation ID (use empty bytes for
    ///   none).
    /// * `salt` - Salt for uniqueness (use empty bytes for default).
    /// * `delay` - The delay in seconds before the operation can be executed.
    /// * `proposer` - The address proposing the operation (must have proposer
    ///   role).
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
    /// **Note**: This function is only for executing operations on external
    /// contracts. For self-administration operations (where target is this
    /// timelock contract), call the admin function directly instead.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `target` - The target contract address.
    /// * `function` - The function name to invoke.
    /// * `args` - The arguments to pass to the function.
    /// * `predecessor` - The predecessor operation ID.
    /// * `salt` - Salt for uniqueness.
    /// * `executor` - The address executing the operation (must have executor
    ///   role if Some).
    ///
    /// # Returns
    ///
    /// The return value from the executed operation.
    ///
    /// # Notes
    ///
    /// * If executors are configured (EXECUTOR_ROLE has members), authorization
    ///   for `executor` is required and the executor must have the
    ///   EXECUTOR_ROLE.
    /// * If no executors are configured, anyone can execute ready operations.
    pub fn execute_op(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
        executor: Option<Address>,
    ) -> Val {
        if get_role_member_count(e, &EXECUTOR_ROLE) != 0 {
            let executor = executor.expect("to be present");
            ensure_role(e, &EXECUTOR_ROLE, &executor);
            executor.require_auth();
        }

        let operation = Operation { target, function, args, predecessor, salt };
        execute_operation(e, &operation)
    }

    /// Cancels a scheduled operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `operation_id` - The unique identifier of the operation to cancel.
    /// * `canceller` - The address cancelling the operation (must have
    ///   canceller role).
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
    /// * `new_delay` - The new minimum delay in seconds.
    ///
    /// # Notes
    ///
    /// * Authorization for `admin` is required.
    /// * This function should typically be called through the timelock itself
    ///   (self-administration) to ensure transparency.
    #[only_admin]
    pub fn update_delay(e: &Env, new_delay: u32) {
        timelock_set_min_delay(e, new_delay);
    }

    /// Returns the minimum delay in seconds required for operations.
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

    /// Returns the timestamp at which an operation becomes ready.
    pub fn get_timestamp(e: &Env, operation_id: BytesN<32>) -> u64 {
        get_timestamp(e, &operation_id)
    }

    /// Returns the current state of an operation.
    pub fn get_operation_state(e: &Env, operation_id: BytesN<32>) -> OperationState {
        get_operation_state(e, &operation_id)
    }

    /// Returns whether an operation exists (scheduled or done).
    pub fn operation_exists(e: &Env, operation_id: BytesN<32>) -> bool {
        operation_exists(e, &operation_id)
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
