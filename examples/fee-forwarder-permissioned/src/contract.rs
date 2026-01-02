//! Fee Forwarder Contract
//!
//! This contract enables fee abstraction by allowing users to pay relayers in
//! tokens instead of native XLM. The contract accepts token payment and
//! forwards calls to target contracts, ensuring atomic execution.
//!
//! ## Flow Overview
//!
//! 1. **User prepares transaction** (off-chain):
//!    - User wants to call `target_contract.target_fn(target_args)`
//!    - User wants to pay the transaction fees with tokens different than XLM
//!      (e.g., USDC)
//!    - User gets a quote from relayer: max fee amount and expiration ledger
//!
//! 2. **User signs authorizations** (first signature):
//!    - User authorizes the fee-forwarder contract with these parameters:
//!      - `fee_token`: Which token to use for payment
//!      - `max_fee_amount`: Maximum fee they're willing to pay
//!      - `expiration_ledger`: When the authorization expires
//!      - `target_contract`, `target_fn`, `target_args`: The actual call to
//!        make
//!      - If `target_contract.target_fn(target_args)` requires additional
//!        authorization, user includes a subinvocation for it.
//!      - If an approval is needed, user authorizes
//!        `fee_token.approve(fee_forwarder, max_fee_amount,
//!      expiration_ledger)` as a subinvocation.
//!
//!    **Note**:
//!    - User does NOT sign the exact `fee_amount` or `relayer` address yet
//!      (these are unknown at signing time)
//!
//! 3. **Relayer picks up transaction** (off-chain):
//!    - Relayer calculates actual `fee_amount` based on current network
//!      conditions
//!
//! 4. **Relayer signs authorization** (second signature):
//!    - Relayer authorizes the fee-forwarder contract
//!    - Relayer must have `executor` role to call `forward()`
//!
//! 5. **Relayer submits transaction**:
//!    - Relayer pays native XLM fees for network inclusion
//!    - Transaction contains call to `fee_forwarder.forward()` with both
//!      authorizations
//!
//! 6. **Contract executes atomically**:
//!    - Validates both user and relayer authorizations
//!    - User approves contract to spend up to `max_fee_amount` tokens
//!    - Contract transfers exactly `fee_amount` tokens from user to itself
//!    - Contract forwards call to `target_contract.target_fn(target_args)`
//!    - If any step fails, entire transaction reverts (including token
//!      transfer)
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Val, Vec};
use stellar_access::access_control::{grant_role_no_auth, set_admin, AccessControl};
use stellar_fee_abstraction::{
    collect_fee_and_invoke, set_allowed_fee_token, sweep_token, FeeAbstractionApproval,
};
use stellar_macros::only_role;

const MANAGER_ROLE: Symbol = symbol_short!("manager");
const EXECUTOR_ROLE: Symbol = symbol_short!("executor");

#[contract]
pub struct FeeForwarder;

#[contractimpl]
impl FeeForwarder {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, executors: Vec<Address>) {
        set_admin(e, &admin);

        grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);

        for executor in executors.iter() {
            grant_role_no_auth(e, &executor, &EXECUTOR_ROLE, &admin);
        }
    }

    /// This function can be invoked only with authorizatons from both sides:
    /// user and relayer.
    #[only_role(relayer, "executor")]
    pub fn forward(
        e: &Env,
        fee_token: Address,
        fee_amount: i128,
        max_fee_amount: i128,
        expiration_ledger: u32,
        target_contract: Address,
        target_fn: Symbol,
        target_args: Vec<Val>,
        user: Address,
        relayer: Address,
    ) -> Val {
        collect_fee_and_invoke(
            e,
            &fee_token,
            fee_amount,
            max_fee_amount,
            expiration_ledger,
            &target_contract,
            &target_fn,
            &target_args,
            &user,
            &e.current_contract_address(), // current contract collects fee
            FeeAbstractionApproval::Lazy,
        )
    }

    #[only_role(operator, "manager")]
    pub fn enable_fee_token(e: &Env, token: Address, operator: Address) {
        set_allowed_fee_token(e, &token, true);
    }

    #[only_role(operator, "manager")]
    pub fn disable_fee_token(e: &Env, token: Address, operator: Address) {
        set_allowed_fee_token(e, &token, false);
    }

    #[only_role(operator, "manager")]
    pub fn sweep_tokens(e: &Env, token: Address, recipient: Address, operator: Address) -> i128 {
        sweep_token(e, &token, &recipient)
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for FeeForwarder {}
