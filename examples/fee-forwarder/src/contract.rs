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
//! 2. **User signs authorization** (first signature):
//!    - User authorizes the fee-forwarder contract with these parameters:
//!      - `fee_token`: Which token to use for payment
//!      - `max_fee_amount`: Maximum fee they're willing to pay
//!      - `expiration_ledger`: When the authorization expires
//!      - `target_contract`, `target_fn`, `target_args`: The actual call to
//!        make
//!    - User MUST include at least one subinvocation:
//!      - `fee_token.approve(fee_forwarder, max_fee_amount, expiration_ledger)`
//!    - Depending on the target contract call, user may need to include a
//!      second subinvocation:
//!      - `target_contract.target_fn(target_args)` (if it requires some
//!        authorization)
//!
//!    - **Note**: User does NOT sign the exact `fee_amount` or `relayer`
//!      address yet (these are unknown at signing time)
//!
//! 3. **Relayer picks up transaction** (off-chain):
//!    - Relayer calculates actual `fee_amount` based on current network
//!      conditions
//!    - Relayer verifies `fee_amount <= max_fee_amount`
//!
//! 4. **Relayer signs authorization** (second signature):
//!    - Relayer authorizes the fee-forwarder contract with these parameters:
//!      - `fee_token`: Same token as user specified
//!      - `fee_amount`: Exact fee to charge (≤ max_fee_amount)
//!      - `target_contract`, `target_fn`, `target_args`: Same as user specified
//!      - `user`: The user's address
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
//!
//! ## Authorization Summary
//!
//! **User authorizes** (signs first, before knowing relayer or exact fee):
//! - `fee_token`, `max_fee_amount`, `expiration_ledger`
//! - `target_contract`, `target_fn`, `target_args`
//!
//! **Relayer authorizes** (signs second, with exact fee):
//! - `fee_token`, `fee_amount` (exact amount ≤ max)
//! - `target_contract`, `target_fn`, `target_args`
//! - `user` (whose transaction is being relayed)
//!
//! ## Security Properties
//!
//! - User can't be charged more than `max_fee_amount`
//! - User's authorization expires at `expiration_ledger`
//! - Only whitelisted relayers (with `executor` role) can call `forward()`
//! - Token transfer and target call are atomic (both succeed or both fail)
//! - Relayer can't change the target call parameters signed by user

#![allow(clippy::too_many_arguments)]
use soroban_sdk::{
    contract, contractimpl, symbol_short, token::TokenClient, Address, Env, IntoVal, Symbol, Val,
    Vec,
};
use stellar_access::access_control::{grant_role_no_auth, set_admin, AccessControl};
use stellar_macros::{default_impl, has_role};

const EXECUTOR_ROLE: Symbol = symbol_short!("executor");

#[contract]
pub struct FeeForwarder;

#[contractimpl]
impl FeeForwarder {
    pub fn __constructor(e: &Env, admin: Address, executors: Vec<Address>) {
        set_admin(e, &admin);

        for executor in executors.iter() {
            grant_role_no_auth(e, &executor, &EXECUTOR_ROLE, &admin);
        }
    }

    /// This function can be invoked only with authorizatons from both sides:
    /// user and relayer.
    #[has_role(relayer, "executor")]
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
        // TODO: check max_fee_amount >= fee_amount
        // TODO: check fee_token is allowed

        // user and relayer authorize each the args that concern them, e.g. user is the
        // 1st to sign the authorizatons, but at that moment they don't know the
        // precise fee they will be charged and the address of the relayer who
        // will sponsor the transaction.

        let user_args_for_auth = (
            fee_token.clone(),
            max_fee_amount,
            expiration_ledger,
            target_contract.clone(),
            target_fn.clone(),
            target_args.clone(),
        )
            .into_val(e);
        user.require_auth_for_args(user_args_for_auth);

        let relayer_args_for_auth = (
            fee_token.clone(),
            fee_amount,
            target_contract.clone(),
            target_fn.clone(),
            target_args.clone(),
            user.clone(),
        )
            .into_val(e);
        relayer.require_auth_for_args(relayer_args_for_auth);

        let token_client = TokenClient::new(e, &fee_token);
        // user signs an approval for `max_fee_amount` so that this contract can charge
        // <= `max_fee_amount`
        token_client.approve(
            &user,
            &e.current_contract_address(),
            &max_fee_amount,
            &expiration_ledger,
        );

        token_client.transfer_from(
            &e.current_contract_address(),
            &user,
            &e.current_contract_address(),
            &fee_amount,
        );

        e.invoke_contract::<Val>(&target_contract, &target_fn, target_args)
    }

    // TODO: more functions to sweep tokens
    // TODO: allow/disallow tokens
}

#[default_impl]
#[contractimpl]
impl AccessControl for FeeForwarder {}
