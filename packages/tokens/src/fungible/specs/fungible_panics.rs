use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{is_auth, nondet_address};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{Base, FungibleToken};

#[rule]
// transfer panics if from does not auth
// status: verified
pub fn transfer_panics_if_unauthorized(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(from.clone()));
    Base::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if not enough balance
// status: verified
pub fn transfer_panics_if_not_enough_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = Base::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount);
    Base::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if amount < 0
// status: verified
pub fn transfer_panics_if_amount_less_than_zero(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    Base::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if spender does not auth
// status: verified
pub fn transfer_from_panics_if_spender_unauthorized(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(spender.clone()));
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if not enough balance
// status: verified
pub fn transfer_from_panics_if_not_enough_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = Base::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if not enough allowance and spender != from
// status: bug
pub fn transfer_from_panics_if_not_enough_allowance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    let allowance = Base::allowance(&e, &from, &spender);
    clog!(allowance);
    cvlr_assume!(allowance < amount);
    cvlr_assume!(spender != from);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if amount < 0
// status: verified
pub fn transfer_from_panics_if_amount_less_than_zero(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// approve panics if owner does not auth
// status: verified
pub fn approve_panics_if_unauthorized(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(owner.clone()));
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// approve panics if amount < 0
// status: verified
pub fn approve_panics_if_amount_less_than_zero(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// approve panics if live_until_ledger > max_ledger
// status: verified
pub fn approve_panics_if_live_until_ledger_greater_than_max_ledger(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(live_until_ledger > e.ledger().max_live_until_ledger());
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// approve panics if live_until_ledger < current_ledger & amount > 0
// status: verified
pub fn approve_panics_if_live_until_ledger_less_than_current_ledger(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(live_until_ledger < e.ledger().sequence());
    cvlr_assume!(amount > 0);
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(false);
}
