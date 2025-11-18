use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;


use soroban_sdk::{Env};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

// invariant: owner != None -> holds in all cases except for renounce_ownership

#[rule]
pub fn after_constructor_owner_is_set(e: Env) {
    let new_owner = nondet_address();
    FVHarnessOwnableContract::__constructor(&e, new_owner);
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assert!(owner != None);
}

#[rule]
pub fn after_transfer_ownership_pending_owner_is_set(e: Env) {

    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assume!(owner != None);

    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    FVHarnessOwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);

    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assert!(owner != None);
}

#[rule]
pub fn after_accept_ownership_owner_is_set(e: Env) {
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assume!(owner != None);
    FVHarnessOwnableContract::accept_ownership(&e);
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assert!(owner != None);
}

#[rule]
pub fn after_owner_restricted_function_owner_is_set(e: Env) {
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assume!(owner != None);
    FVHarnessOwnableContract::owner_restricted_function(&e);
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assert!(owner != None);
}

// the invariant for the case renounce_ownership obviously does not work.


// invariant: pending_owner != none -> owner != none

#[rule]
pub fn after_constructor_pending_owner_implies_owner(e: Env) {
}