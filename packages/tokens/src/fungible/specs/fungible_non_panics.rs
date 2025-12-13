use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{is_auth, nondet_address};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{
    storage::{AllowanceData, AllowanceKey, StorageKey},
    Base, FungibleToken,
};
// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to
// consider also panicking paths.

pub fn storage_setup_balance(e: Env, account: Address) {
    let balance: i128 = nondet();
    e.storage().persistent().set(&StorageKey::Balance(account), &balance);
}

pub fn storage_setup_allowance(e: Env, owner: Address, spender: Address) {
    let amount: i128 = nondet();
    let live_until_ledger: u32 = nondet();
    let allowance = AllowanceData { amount, live_until_ledger };
    e.storage()
        .temporary()
        .set(&StorageKey::Allowance(AllowanceKey { owner, spender }), &allowance);
}

#[rule]
// requires
// from auth
// from has enough balance
// amount >= 0
// status: violation - problem with storage and nondet?
pub fn transfer_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(from.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let from_balance = Base::balance(&e, &from);
    clog!(from_balance);
    cvlr_assume!(from_balance >= amount);
    cvlr_assume!(amount >= 0);
    Base::transfer(&e, &from, &to, amount);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn transfer_non_panic_sanity(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(from.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let from_balance = Base::balance(&e, &from);
    clog!(from_balance);
    cvlr_assume!(from_balance >= amount);
    cvlr_assume!(amount >= 0);
    Base::transfer(&e, &from, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// spender auth
// from has enough allowance
// spender has enough allowance
// amount >= 0
// status: violation - problem with storage and nondet?
pub fn transfer_from_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(spender.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let balance_from = Base::balance(&e, &from);
    clog!(balance_from);
    cvlr_assume!(balance_from >= amount);
    storage_setup_allowance(e.clone(), from.clone(), spender.clone());
    let allowance_spender = Base::allowance(&e, &from, &spender);
    clog!(allowance_spender);
    cvlr_assume!(allowance_spender >= amount);
    cvlr_assume!(amount >= 0);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn transfer_from_non_panic_sanity(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(spender.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let balance_from = Base::balance(&e, &from);
    clog!(balance_from);
    cvlr_assume!(balance_from >= amount);
    storage_setup_allowance(e.clone(), from.clone(), spender.clone());
    let allowance_spender = Base::allowance(&e, &from, &spender);
    clog!(allowance_spender);
    cvlr_assume!(allowance_spender >= amount);
    cvlr_assume!(amount >= 0);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// owner auth
// amount >= 0
// valid live until ledger
// status: verified
pub fn approve_non_panic(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger = nondet();
    clog!(live_until_ledger);
    cvlr_assume!(is_auth(owner.clone()));
    cvlr_assume!(amount >= 0);
    let current_ledger = e.ledger().sequence();
    let max_live_until_ledger = e.ledger().max_live_until_ledger();
    let non_zero_amount = amount > 0;
    let ledger_more_than_max = live_until_ledger > max_live_until_ledger;
    let ledger_less_than_current = live_until_ledger < current_ledger;
    cvlr_assume!(!ledger_more_than_max);
    cvlr_assume!(!(non_zero_amount && ledger_less_than_current));
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn approve_non_panic_sanity(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger = nondet();
    clog!(live_until_ledger);
    cvlr_assume!(is_auth(owner.clone()));
    cvlr_assume!(amount >= 0);
    let current_ledger = e.ledger().sequence();
    let max_live_until_ledger = e.ledger().max_live_until_ledger();
    let non_zero_amount = amount > 0;
    let ledger_more_than_max = live_until_ledger > max_live_until_ledger;
    let ledger_less_than_current = live_until_ledger < current_ledger;
    cvlr_assume!(!ledger_more_than_max);
    cvlr_assume!(!(non_zero_amount && ledger_less_than_current));
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_satisfy!(true);
}
