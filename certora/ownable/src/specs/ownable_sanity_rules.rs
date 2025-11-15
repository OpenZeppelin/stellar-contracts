use cvlr::{cvlr_assert};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

#[rule]
pub fn get_owner_sanity(e: Env) {
    let owner = nondet_address();
    FVHarnessOwnableContract::__constructor(&e, owner);
    let _ =FVHarnessOwnableContract:: get_owner(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn set_owner_sanity(e: Env) {
    let owner = nondet_address();
    FVHarnessOwnableContract::__constructor(&e, owner);
    cvlr_assert!(false);
}

#[rule]
pub fn transfer_ownership_sanity(e: Env) {
    let owner = nondet_address();
    FVHarnessOwnableContract::__constructor(&e, owner);
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    FVHarnessOwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
pub fn accept_ownership_sanity(e: Env) {
    let owner = nondet_address();
    FVHarnessOwnableContract::__constructor(&e, owner);
    FVHarnessOwnableContract::accept_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn renounce_ownership_sanity(e: Env) {
    let owner = nondet_address();
    FVHarnessOwnableContract::__constructor(&e, owner);
    FVHarnessOwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}