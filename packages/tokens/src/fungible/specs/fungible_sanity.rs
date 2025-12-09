use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{Base, FungibleToken};

#[rule]
pub fn total_supply_sanity(e: Env) {
    let _ = Base::total_supply(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn balance_sanity(e: Env) {
    let account = nondet_address();
    let _ = Base::balance(&e, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allowance_sanity(e: Env) {
    let owner = nondet_address();
    let spender = nondet_address();
    let _ = Base::allowance(&e, &owner, &spender);
    cvlr_satisfy!(true);
}

#[rule]
pub fn transfer_sanity(e: Env) {
    let to = nondet_address();
    let from = nondet_address();
    let amount = nondet();
    Base::transfer(&e, &from, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn transfer_from_sanity(e: Env) {
    let spender = nondet_address();
    let to = nondet_address();
    let from = nondet_address();
    let amount = nondet();
    Base::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn approve_sanity(e: Env) {
    let owner = nondet_address();
    let spender = nondet_address();
    let amount = nondet();
    let until = nondet();
    Base::approve(&e, &owner, &spender, amount, until);
    cvlr_satisfy!(true);
}

#[rule]
pub fn decimals_sanity(e: Env) {
    Base::decimals(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn name_sanity(e: Env) {
    Base::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn symbol_sanity(e: Env) {
    Base::symbol(&e);
    cvlr_satisfy!(true);
}
