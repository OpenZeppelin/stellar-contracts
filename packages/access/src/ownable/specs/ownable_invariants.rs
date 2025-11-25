use cvlr::{cvlr_assert, cvlr_assume,cvlr_satisfy};
use cvlr_soroban::{nondet_address};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;


use soroban_sdk::{Env};
use cvlr::clog;
use crate::ownable::{specs::{helper::get_pending_owner, ownable_contract::OwnableContract}, *};

// invariant: owner != None -> holds in all cases except for renounce_ownership

// helpers
pub fn assume_pre_owner_is_set(e: Env) {
    let owner_pre = OwnableContract::get_owner(&e);
    cvlr_assume!(owner_pre.is_some());
}

pub fn assert_post_owner_is_set(e: Env) {
    let owner_post = OwnableContract::get_owner(&e);
    cvlr_assert!(owner_post.is_some());
}

/////////
#[rule]
// status: verified
pub fn after_constructor_owner_is_set(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::ownable_constructor(&e, new_owner);
    assert_post_owner_is_set(e);
}

#[rule]
// status: verified
pub fn after_constructor_owner_is_set_sanity(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::ownable_constructor(&e, new_owner);
    cvlr_satisfy!(true);
}

/////////
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
pub fn after_transfer_ownership_pending_owner_is_set_sanity(e: Env) {
    assume_pre_owner_is_set(e.clone());
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_satisfy!(true);
}

/////////
#[rule]
// status: verified
pub fn after_accept_ownership_owner_is_set(e: Env) {
    assume_pre_owner_is_set(e.clone());
    OwnableContract::accept_ownership(&e);
    assert_post_owner_is_set(e);
}

#[rule]
// status: verified
pub fn after_accept_ownership_owner_is_set_sanity(e: Env) {
    assume_pre_owner_is_set(e.clone());
    OwnableContract::accept_ownership(&e);
    cvlr_satisfy!(true)
}

// for the case renounce_ownership it's obviously true - and expected

/////////
#[rule]
// status: verified
pub fn after_owner_restricted_function_owner_is_set(e: Env) {
    assume_pre_owner_is_set(e.clone());
    OwnableContract::owner_restricted_function(&e);
    assert_post_owner_is_set(e);
}

#[rule]
// status: verified
pub fn after_owner_restricted_function_owner_is_set_sanity(e: Env) {
    assume_pre_owner_is_set(e.clone());
    OwnableContract::owner_restricted_function(&e);
    cvlr_satisfy!(true)
}

// invariant: pending_owner != none implies owner != none

// helpers
pub fn assume_pre_pending_owner_implies_owner(e: &Env) {
    let pending_owner_pre = get_pending_owner(e);
    if let Some(pend_pre) = pending_owner_pre.clone() {
        clog!(cvlr_soroban::Addr(&pend_pre));
    }
    let owner = OwnableContract::get_owner(&e);
    if let Some(owner_internal_pre) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal_pre));
    }
    if pending_owner_pre.is_some() {
        cvlr_assume!(owner.is_some());
    }
}

pub fn assert_post_pending_owner_implies_owner(e: &Env) {
    let pending_owner_post = get_pending_owner(&e);
    if let Some(pend_post) = pending_owner_post.clone() {
        clog!(cvlr_soroban::Addr(&pend_post));
    }
    let owner = OwnableContract::get_owner(&e);
    if let Some(owner_internal_post) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal_post));
    }
    if pending_owner_post.is_some() {
        cvlr_assert!(owner.is_some());
    }
}

/////////
#[rule]
// status: verified
pub fn after_constructor_pending_owner_implies_owner(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::ownable_constructor(&e, new_owner);
    assert_post_pending_owner_implies_owner(&e);
}

#[rule]
// status: verified
pub fn after_constructor_pending_owner_implies_owner_sanity(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::ownable_constructor(&e, new_owner);
    cvlr_satisfy!(true);
}


/////////
#[rule]
// status: verified
pub fn after_transfer_ownership_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    assert_post_pending_owner_implies_owner(&e);
}

#[rule]
// status: verified
pub fn after_transfer_ownership_pending_owner_implies_owner_sanity(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_satisfy!(true);
}

/////////
#[rule]
// status: verified
pub fn after_accept_ownership_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::accept_ownership(&e);
    assert_post_pending_owner_implies_owner(&e);
}

#[rule]
// status: verified
pub fn after_accept_ownership_pending_owner_implies_owner_sanity(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::accept_ownership(&e);
    cvlr_satisfy!(true);
}

/////////
#[rule]
// status: verified
pub fn after_renounce_ownership_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::renounce_ownership(&e);
    assert_post_pending_owner_implies_owner(&e);
}

#[rule]
// status: verified
pub fn after_renounce_ownership_pending_owner_implies_owner_sanity(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::renounce_ownership(&e);
    cvlr_satisfy!(true)
}

/////////
#[rule]
// status: verified
pub fn after_owner_restricted_function_pending_owner_implies_owner(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::owner_restricted_function(&e);
    assert_post_pending_owner_implies_owner(&e);
}

#[rule]
// status: verified
pub fn after_owner_restricted_function_pending_owner_implies_owner_sanity(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::owner_restricted_function(&e);
    cvlr_satisfy!(true);
}