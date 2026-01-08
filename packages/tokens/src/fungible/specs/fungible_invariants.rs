use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{Base, FungibleToken};

// helper assumption -- this is an invariant that we cannot
// prove without ghosts and hooks
pub fn assume_balance_leq_total_supply(e: &Env, account: &Address) {
    let balance = Base::balance(e, account);
    clog!(balance);
    let total_supply = Base::total_supply(e);
    clog!(total_supply);
    cvlr_assume!(balance <= total_supply);
}

// invariant: total_supply >= 0

pub fn assume_pre_total_supply_geq_zero(e: &Env) {
    let total_supply = Base::total_supply(e);
    clog!(total_supply);
    cvlr_assume!(total_supply >= 0);
}

pub fn assert_post_total_supply_geq_zero(e: &Env) {
    let total_supply = Base::total_supply(e);
    clog!(total_supply);
    cvlr_assert!(total_supply >= 0);
}


// rules 

#[rule]
// status: verified
pub fn fungible_after_transfer_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount = nondet();
    clog!(amount);
    Base::transfer(&e, &from, &to, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn fungible_after_transfer_from_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount = nondet();
    clog!(amount);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn fungible_after_approve_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount = nondet();
    clog!(amount);
    let live_until_ledger = nondet();
    clog!(live_until_ledger);
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn fungible_after_mint_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let amount = nondet();
    clog!(amount);
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    Base::mint(&e, &owner, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn fungible_after_burn_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let amount = nondet();
    clog!(amount);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    Base::burn(&e, &from, amount);
    assume_balance_leq_total_supply(&e, &from); // special assumption for this case
    assert_post_total_supply_geq_zero(&e);
}

// invariant: balance >= 0

pub fn assume_pre_balance_geq_zero(e: &Env, account: &Address) {
    let balance = Base::balance(e, account);
    clog!(balance);
    cvlr_assume!(balance >= 0);
}

pub fn assert_post_balance_geq_zero(e: &Env, account: &Address) {
    let balance = Base::balance(e, account);
    clog!(balance);
    cvlr_assert!(balance >= 0);
}

// rules

#[rule]
// status: verified
pub fn fungible_after_transfer_balance_geq_zero(e: Env) {
    let account = nondet_address();
    assume_pre_balance_geq_zero(&e, &account);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    Base::transfer(&e, &from, &to, amount);
    assert_post_balance_geq_zero(&e, &account);
}

#[rule]
// status: verified
pub fn fungible_after_transfer_from_balance_geq_zero(e: Env) {
    let account = nondet_address();
    assume_pre_balance_geq_zero(&e, &account);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    assert_post_balance_geq_zero(&e, &account);
}

#[rule]
// status: verified
pub fn fungible_after_approve_balance_geq_zero(e: Env) {
    let account = nondet_address();
    assume_pre_balance_geq_zero(&e, &account);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount = nondet();
    clog!(amount);
    let live_until_ledger = nondet();
    clog!(live_until_ledger);
    Base::approve(&e, &account, &spender, amount, live_until_ledger);
    assert_post_balance_geq_zero(&e, &account);
}

#[rule]
// status: verified
pub fn fungible_after_mint_balance_geq_zero(e: Env) {
    let account = nondet_address();
    assume_pre_balance_geq_zero(&e, &account);
    let amount = nondet();
    clog!(amount);
    Base::mint(&e, &account, amount);
    assert_post_balance_geq_zero(&e, &account);
}

#[rule]
// status: verified
pub fn fungible_after_burn_balance_geq_zero(e: Env) {
    let account = nondet_address();
    assume_pre_balance_geq_zero(&e, &account);
    let amount = nondet();
    clog!(amount);
    Base::burn(&e, &account, amount);
    assert_post_balance_geq_zero(&e, &account);
}