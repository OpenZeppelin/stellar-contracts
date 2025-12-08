use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr::clog;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use crate::fungible::FungibleToken;
use crate::fungible::Base;

// todo: total_supply does not change other than mint.
// todo (?) total_supply >= balance(a1)+balance(a2)

// maybe its not right to talk about invariants just for fungible because its not really a contract setting (?) 
// or maybe its fine

// helpers
pub fn assume_pre_total_supply_geq_balance(e: Env, account: &Address) {
    clog!(cvlr_soroban::Addr(account));
    let total_supply = Base::total_supply(&e);
    clog!(total_supply);
    let balance = Base::balance(&e, account);
    clog!(balance);
    cvlr_assume!(total_supply >= balance);
}

pub fn assert_post_total_supply_geq_balance(e: Env, account: &Address) {
    clog!(cvlr_soroban::Addr(account));
    let total_supply = Base::total_supply(&e);
    clog!(total_supply);    
    let balance = Base::balance(&e, account);
    clog!(balance);
    cvlr_assert!(total_supply >= balance);
}

#[rule]
// status: violated - spurious 
// https://prover.certora.com/output/5771024/7ac81c9f026e44b1a29a116052a06333/
pub fn after_transfer_total_supply_geq_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount = nondet();
    clog!(amount);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    Base::transfer(&e, &from, &to, amount);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: violated - seems spurious
pub fn after_transfer_from_total_supply_geq_balance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount = nondet();
    clog!(amount);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: verified
pub fn after_approve_total_supply_geq_balance(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount = nondet();
    clog!(amount);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    let live_until_ledger = nondet();
    clog!(live_until_ledger);
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    assert_post_total_supply_geq_balance(e, &account);
}