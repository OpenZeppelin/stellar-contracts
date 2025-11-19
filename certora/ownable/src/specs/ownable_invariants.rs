use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;


use soroban_sdk::{Env};

use stellar_access::ownable::*;

use crate::ownable_contract::OwnableContract;
use crate::specs::helper::get_pending_owner;

// invariant: owner != None -> holds in all cases except for renounce_ownership

// helpers
pub fn assume_pre_owner_is_set(e: Env) {
    let owner_pre = OwnableContract::get_owner(&e);
    cvlr_assume!(owner_pre != None);
}

pub fn assert_post_owner_is_set(e: Env) {
    let owner_post = OwnableContract::get_owner(&e);
    cvlr_assert!(owner_post != None);
}

#[rule]
// status: verified
pub fn after_constructor_owner_is_set(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::__constructor(&e.clone(), new_owner);
    assert_post_owner_is_set(e);
}

#[rule]
// status: verified
pub fn after_transfer_ownership_pending_owner_is_set(e: Env) {
    assume_pre_owner_is_set(e.clone());
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    assert_post_owner_is_set(e);
}

#[rule]
// status: verified
pub fn after_accept_ownership_owner_is_set(e: Env) {
    assume_pre_owner_is_set(e.clone());
    OwnableContract::accept_ownership(&e);
    assert_post_owner_is_set(e);
}

// for the case renounce_ownership obviously does not work -- and this is fine.

#[rule]
// status: verified
pub fn after_owner_restricted_function_owner_is_set(e: Env) {
    assume_pre_owner_is_set(e.clone());
    OwnableContract::owner_restricted_function(&e);
    assert_post_owner_is_set(e);
}

// invariant: pending_owner != none implies owner != none

// helpers
pub fn assume_pre_pending_owner_implies_owner(e: Env) {
    let pending_owner = get_pending_owner(&e);
    let owner = OwnableContract::get_owner(&e);
    if let Some(_) = pending_owner.clone() {
        cvlr_assume!(owner != None);
    }
}

pub fn assert_post_pending_owner_implies_owner(e: Env) {
    let pending_owner = get_pending_owner(&e);
    let owner = OwnableContract::get_owner(&e);
    if let Some(_) = pending_owner.clone() {
        cvlr_assert!(owner != None);
    }
}

#[rule]
// status: verified
pub fn after_constructor_pending_owner_implies_owner(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::__constructor(&e, new_owner);
    assert_post_pending_owner_implies_owner(e);
}

#[rule]
// status: verified
pub fn after_transfer_ownership_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(e.clone());
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    assert_post_pending_owner_implies_owner(e);
}

#[rule]
// status: vacuity issue!
pub fn after_accept_ownership_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(e.clone());
    OwnableContract::accept_ownership(&e);
    assert_post_pending_owner_implies_owner(e);
}

#[rule]
// status: vacuity issue!
pub fn after_renounce_ownership_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(e.clone());
    OwnableContract::renounce_ownership(&e);
    assert_post_pending_owner_implies_owner(e);
}

#[rule]
// status: verified
pub fn after_owner_restricted_function_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(e.clone());
    OwnableContract::owner_restricted_function(&e);
    assert_post_pending_owner_implies_owner(e);
}