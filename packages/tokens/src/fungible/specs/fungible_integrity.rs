use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{Base, FungibleToken};

#[rule]
// transfer changes balances accordingly
// status: verified
pub fn transfer_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
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
// transfer_from changes total supply accordingly
// status: verified https://prover.certora.com/output/33158/4631143c82f34fa58591b8f12c1411d3
pub fn transfer_from_integrity_1(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();

    let total_supply_pre = Base::total_supply(&e);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    let total_supply_post = Base::total_supply(&e);
    cvlr_assert!(total_supply_post == total_supply_pre);
}

#[rule]
// transfer_from changes balances and allowance accordingly
// status: verified https://prover.certora.com/output/33158/4f0610d39049484db39a3d270a5038c2
pub fn transfer_from_integrity_2(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();

    let balance_from_pre = Base::balance(&e, &from);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    let balance_from_post = Base::balance(&e, &from);

    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - amount);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// transfer_from changes balances and allowance accordingly
// status: verified https://prover.certora.com/output/33158/4661dcb42134455ab9c1e47d9a7887c9
pub fn transfer_from_integrity_3(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();

    let balance_to_pre = Base::balance(&e, &to);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    let balance_to_post = Base::balance(&e, &to);

    if to != from {
        cvlr_assert!(balance_to_post == balance_to_pre + amount);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// transfer_from changes balances and allowance accordingly
// status: violated: https://prover.certora.com/output/33158/0180ad7c7e534d6cbc950a393201775c
// not sure if this is even true because `allowance` has an additional check that `allowance_data` does not.
// see changes made below.
pub fn transfer_from_integrity_4(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    
    let allowance_pre = Base::allowance(&e, &from, &spender);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    let allowance_post = Base::allowance(&e, &from, &spender);
    // let allowance_pre = Base::allowance_data(&e, &from, &spender).amount;
    // clog!(allowance_pre);

    // let allowance = Base::allowance_data(&e, &from, &spender);
    // let allowance_post = allowance.amount;
    // let until = allowance.live_until_ledger;
    // let rhs = e.ledger().sequence();
    // clog!(allowance_post);
    // clog!(until);
    // clog!(rhs);

    // cvlr_assume!(amount > 0);
    if spender != from {
        cvlr_assert!(allowance_post == allowance_pre - amount);
    } else {
        cvlr_assert!(allowance_post == allowance_pre);
    }
}

#[rule]
// approve changes allowance accordingly
// status: verified
pub fn approve_integrity(e: Env) {
    // note - the allowance and approve are all in the same env.
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    let allowance_pre = Base::allowance(&e, &owner, &spender);
    clog!(allowance_pre);
    Base::approve(&e, &owner, &spender, amount, live_until_ledger);
    let allowance_post = Base::allowance(&e, &owner, &spender);
    clog!(allowance_post);
    cvlr_assert!(allowance_post == amount);
}
