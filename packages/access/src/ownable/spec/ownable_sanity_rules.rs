
use cvlr::{cvlr_assert};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};

use crate::ownable::*;

#[rule]
pub fn get_owner_sanity(e: Env) {
    let _ = get_owner(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn set_owner_sanity(e: Env) {
    let owner = nondet_address();
    set_owner(&e, &owner);
    cvlr_assert!(false);
}

#[rule]
pub fn transfer_ownership_sanity(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    transfer_ownership(&e, &new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
pub fn accept_ownership_sanity(e: Env) {
    accept_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn renounce_ownership_sanity(e: Env) {
    renounce_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn enforce_owner_auth_sanity(e: Env) {
    enforce_owner_auth(&e);
    cvlr_assert!(false);
}
