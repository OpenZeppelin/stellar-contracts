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
//!    **Note**:
//!    - User does NOT sign the exact `fee_amount` or `relayer` address yet
//!      (these are unknown at signing time)
//!    - Pay attention to the composition of authorization entries (compare with
//!      "examples/fee-forwarder-pemissioned")
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
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Val, Vec};
use stellar_fee_abstraction::{invoke_then_collect_fee, FeeAbstractionApproval};

#[contract]
pub struct FeeForwarder;

#[contractimpl]
impl FeeForwarder {
    /// This function can be invoked only with authorizatons from both sides:
    /// user and relayer.
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
        relayer.require_auth();

        // Depending on whether we first invoke target and than collect fee,
        // composing authorization entries might differ. Compare "test.rs" from this
        // crate with "examples/fee-forwarder-pemissioned".
        invoke_then_collect_fee(
            e,
            &fee_token,
            fee_amount,
            max_fee_amount,
            expiration_ledger,
            &target_contract,
            &target_fn,
            &target_args,
            &user,
            &relayer, // relayer collects fee
            FeeAbstractionApproval::Lazy,
        )
    }
}
