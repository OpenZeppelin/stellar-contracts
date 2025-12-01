use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr::clog;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use crate::fungible::FungibleToken;
use crate::fungible::Base;

#[rule]
// transfer changes balances accordingly
// status: verified
pub fn transfer_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    let balance_from_pre = Base::balance(&e, &from);
    clog!(balance_from_pre);
    let balance_to_pre = Base::balance(&e, &to);
    clog!(balance_to_pre);
    let total_supply_pre = Base::total_supply(&e);
    clog!(total_supply_pre);
    Base::transfer(&e, &from, &to, amount);
    let balance_from_post = Base::balance(&e, &from);
    clog!(balance_from_post);
    let balance_to_post = Base::balance(&e, &to);
    clog!(balance_to_post);
    let total_supply_post = Base::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre);
    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - amount);
        cvlr_assert!(balance_to_post == balance_to_pre + amount);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// transfer_from changes balances and allowance accordingly 
// status:
pub fn transfer_from_integrity(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount:i128 = nondet();
    clog!(amount);
    let balance_from_pre = Base::balance(&e, &from);
    clog!(balance_from_pre);
    let balance_to_pre = Base::balance(&e, &to);
    clog!(balance_to_pre);
    let allowance_pre = Base::allowance(&e, &from, &spender);
    clog!(allowance_pre);
    let total_supply_pre = Base::total_supply(&e);
    clog!(total_supply_pre);
    Base::transfer_from(&e, &spender, &from, &to, amount); 
    let balance_from_post = Base::balance(&e, &from);
    clog!(balance_from_post);
    let balance_to_post = Base::balance(&e, &to);
    clog!(balance_to_post);
    let allowance_post = Base::allowance(&e, &from, &spender);
    clog!(allowance_post);
    let total_supply_post = Base::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre);
    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - amount);
        cvlr_assert!(balance_to_post == balance_to_pre + amount);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
    if spender != from {
        cvlr_assert!(allowance_post == allowance_pre - amount);
    } else {
        cvlr_assert!(allowance_post == allowance_pre);
    }
}

#[rule]
// approve changes allowance accordingly
// status:
pub fn approve_integrity(e: Env) {
    // note - the allowance and approve are all in the same env.
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount:i128 = nondet();
    clog!(amount);
    let live_until_ledger:u32 = nondet();
    clog!(live_until_ledger);
    let allowance_pre = Base::allowance(&e, &owner, &spender);
    clog!(allowance_pre);
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    let allowance_post = Base::allowance(&e, &owner, &spender);
    clog!(allowance_post);
    cvlr_assert!(allowance_post == amount);
}