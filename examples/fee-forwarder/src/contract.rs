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
}

#[default_impl]
#[contractimpl]
impl AccessControl for FeeForwarder {}
