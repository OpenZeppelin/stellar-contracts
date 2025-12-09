use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{blocklist::BlockList, Base, FungibleToken};

// ################## INTEGRITY RULES ##################

#[rule]
// block_user sets blocked to true
// status: verified
pub fn block_user_integrity(e: Env) {
    let account = nondet_address();
    BlockList::block_user(&e, &account);
    let blocked_post = BlockList::blocked(&e, &account);
    cvlr_assert!(blocked_post == true);
}

#[rule]
// unblock_user sets blocked to false
// status: verified
pub fn unblock_user_integrity(e: Env) {
    let account = nondet_address();
    BlockList::unblock_user(&e, &account);
    let blocked_post = BlockList::blocked(&e, &account);
    cvlr_assert!(blocked_post == false);
}

// ################## PANIC RULES ##################

#[rule]
// transfer panics if from is blocked
// status: verified
pub fn transfer_panics_if_from_blocked(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(BlockList::blocked(&e, &from));
    BlockList::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if to is blocked
// status: verified
pub fn transfer_panics_if_to_blocked(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(BlockList::blocked(&e, &to));
    BlockList::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if from is blocked
// status: verified
pub fn transfer_from_panics_if_from_blocked(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(BlockList::blocked(&e, &from));
    BlockList::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if to is blocked
// status: verified
pub fn transfer_from_panics_if_to_blocked(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(BlockList::blocked(&e, &to));
    BlockList::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

// spender may be blocked - see mod

#[rule]
// approve panics if owner is blocked
// status: verified
pub fn approve_panics_if_owner_blocked(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(BlockList::blocked(&e, &owner));
    BlockList::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// burn panics if from is blocked
// status: verified
pub fn burn_panics_if_from_blocked(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(BlockList::blocked(&e, &from));
    BlockList::burn(&e, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if from is blocked
// status: verified
pub fn burn_from_panics_if_from_blocked(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(BlockList::blocked(&e, &from));
    BlockList::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(false);
}
