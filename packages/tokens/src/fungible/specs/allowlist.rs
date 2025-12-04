use cvlr::{cvlr_assert, cvlr_satisfy, cvlr_assume, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr::clog;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use crate::fungible::FungibleToken;
use crate::fungible::Base;
use crate::fungible::allowlist::AllowList;

// ################## INTEGRITY RULES ##################

#[rule]
// allow_user sets allowed to true
// status: verified
pub fn allow_user_integrity(e: Env) {
    let account = nondet_address();
    AllowList::allow_user(&e, &account);
    let allowed_post = AllowList::allowed(&e, &account);
    cvlr_assert!(allowed_post == true);
}

#[rule]
// disallow_user sets allowed to false
// status: verified
pub fn disallow_user_integrity(e: Env) {
    let account = nondet_address();
    AllowList::disallow_user(&e, &account);
    let allowed_post = AllowList::allowed(&e, &account);
    cvlr_assert!(allowed_post == false);
}

// ################## PANIC RULES ##################

#[rule]
// transfer panics if from is not allowed
// status: verified
pub fn transfer_panics_if_from_not_allowed(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(!AllowList::allowed(&e, &from));
    AllowList::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}   

#[rule]
// transfer panics if to is not allowed
// status: verified
pub fn transfer_panics_if_to_not_allowed(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(!AllowList::allowed(&e, &to));
    AllowList::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if from is not allowed
// status: verified
pub fn transfer_from_panics_if_from_not_allowed(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(!AllowList::allowed(&e, &from));
    AllowList::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if to is not allowed
// status: verified
pub fn transfer_from_panics_if_to_not_allowed(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(!AllowList::allowed(&e, &to));
    AllowList::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

// spender does not have to be allowed - see mod

#[rule]
// approve panics if owner is not allowed
// status: verified
pub fn approve_panics_if_owner_not_allowed(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount:i128 = nondet();
    clog!(amount);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(!AllowList::allowed(&e, &owner));
    AllowList::approve(&e, &owner, &spender, amount, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// burn panics if from is not allowed
// status: verified
pub fn burn_panics_if_from_not_allowed(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(!AllowList::allowed(&e, &from));
    AllowList::burn(&e, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if from is not allowed
// status: verified
pub fn burn_from_panics_if_from_not_allowed(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount:i128 = nondet();
    clog!(amount);
    cvlr_assume!(!AllowList::allowed(&e, &from));
    AllowList::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(false);
}

