use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;


use soroban_sdk::{Env};

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
    OwnableContract::__constructor(&e, new_owner);
    assert_post_owner_is_set(e);
}

#[rule]
// status: verified
pub fn after_constructor_owner_is_set_sanity(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::__constructor(&e, new_owner);
    cvlr_assert!(false);
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
    cvlr_assert!(false);
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
    cvlr_assert!(false)
}

// for the case renounce_ownership obviously does not work -- and this is fine.

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
    cvlr_assert!(false)
}

// invariant: pending_owner != none implies owner != none

// helpers
pub fn assume_pre_pending_owner_implies_owner(e: &Env) {
    // let pending_owner = get_pending_owner(&e);
    let key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&key);
    let owner = OwnableContract::get_owner(&e);
    if pending_owner.is_some() {
        cvlr_assume!(owner.is_some());
    }
}

pub fn assert_post_pending_owner_implies_owner(e: &Env) {
    // let pending_owner = get_pending_owner(&e);
    let key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&key);
    let owner = OwnableContract::get_owner(&e);
    if pending_owner.is_some() {
        cvlr_assert!(owner.is_some());
    }
}

/////////
#[rule]
// status: verified
pub fn after_constructor_pending_owner_implies_owner(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::__constructor(&e, new_owner);
    assert_post_pending_owner_implies_owner(&e);
}

#[rule]
// status: verified
pub fn after_constructor_pending_owner_implies_owner_sanity(e: Env) {
    let new_owner = nondet_address();
    OwnableContract::__constructor(&e, new_owner);
    cvlr_assert!(false);
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
    cvlr_assert!(false);
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
    cvlr_assert!(false);
}

/////////
#[rule]
// status: violated
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
    cvlr_assert!(false)
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
pub fn after_owner_restricted_function_pending_owner_implies_owner_sanity(e: Env) {
    assume_pre_pending_owner_implies_owner(&e);
    OwnableContract::owner_restricted_function(&e);
    cvlr_assert!(false);
}