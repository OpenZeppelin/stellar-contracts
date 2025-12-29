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
// transfer_from does not change total supply
// status: verified
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
// transfer_from changes the balance of from accordingly
// status: verified
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
// transfer_from changes the balance of to accordingly
// status: verified
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
// transfer_from changes allowance accordingly
// status: bug 
// same bug in transfer_from as observed in transfer_from_panics_if_not_enough_allowance
// as we saw if allowance.amount = amount and live_until_ledger < current_ledger
// you can still transfer_from then:
// allowance_pre = 0 (because allowance expired)
// allowance_post = 0 
// but no panic
// amount > 0 => violation.
pub fn transfer_from_integrity_4(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    clog!(cvlr_soroban::Addr(&spender));
    clog!(cvlr_soroban::Addr(&from));
    clog!(cvlr_soroban::Addr(&to));
    clog!(amount);
    let allowance_pre = Base::allowance(&e, &from, &spender);
    clog!(allowance_pre);
    Base::transfer_from(&e, &spender, &from, &to, amount);
    let allowance_post = Base::allowance(&e, &from, &spender);
    clog!(allowance_post);
    cvlr_assert!(allowance_post == allowance_pre - amount); // allowance to yourself is treated the same way.
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
