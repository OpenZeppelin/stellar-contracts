use cvlr::{cvlr_assert, cvlr_satisfy, cvlr_assume, nondet::*};
use cvlr_soroban::{nondet_address, is_auth};
use cvlr::clog;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use crate::fungible::FungibleToken;
use crate::fungible::Base;

// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to consider also panicking paths.

#[rule]
// requires
// from auth
// from has enough balance
// amount >= 0
// status: wip
pub fn transfer_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(from.clone()));
    let from_balance = Base::balance(&e, &from);
    clog!(from_balance);
    cvlr_assume!(from_balance >= amount);
    cvlr_assume!(amount >= 0);
    Base::transfer(&e, &from, &to, amount);
    cvlr_assert!(true);
}

#[rule]
// requires
// spender auth
// from has enough allowance
// spender has enough allowance
// amount >= 0
// status: wip
pub fn transfer_from_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(spender.clone()));
    let balance_from = Base::balance(&e, &from);
    clog!(balance_from);
    cvlr_assume!(balance_from >= amount);
    let allowance_spender = Base::allowance(&e, &from, &spender);
    clog!(allowance_spender);
    cvlr_assume!(allowance_spender >= amount);
    cvlr_assume!(amount >= 0);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(true);
}

// approve non_panic