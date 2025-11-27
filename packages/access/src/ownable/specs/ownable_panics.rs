use cvlr::{cvlr_assert, cvlr_assume,cvlr_satisfy};
use cvlr_soroban::{nondet_address,is_auth};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;

use soroban_sdk::{Env, Address};

use crate::ownable::specs::ownable_contract::OwnableContract;
use crate::ownable::*;

use crate::ownable::specs::helper::get_pending_owner;

// panic rules should all "pass" to be considered verified, even though they assert false.

// package functions

#[rule]
// transfer_ownership panics if the not authorized by the owner.
// status: verified  
pub fn transfer_ownership_panics_if_unauth_by_owner(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let owner = OwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
        cvlr_assume!(!is_auth(owner_internal));
    }
    OwnableContract::transfer_ownership(&e, new_owner.clone(), live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_ownership panics if the owner is not set.
// status: verified
pub fn transfer_ownership_panics_if_owner_not_set(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    let owner = OwnableContract::get_owner(&e);
    cvlr_assume!(owner.is_none());
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_ownership panics if live_until_ledger = 0 and PendingOwner = None
// status: verified
pub fn transfer_ownership_panics_if_live_until_ledger_0_and_pending_owner_none(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = 0;
    let pending_owner = get_pending_owner(&e);
    cvlr_assume!(pending_owner.is_none());
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_ownership panics if live_until_ledger = 0 and PendingOwner != new_owner
// status: verified
pub fn transfer_ownership_panics_if_live_until_ledger_0_and_diff_pending_owner(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = 0;
    let pending_owner = get_pending_owner(&e);
    if let Some(pending_owner_internal) = pending_owner.clone() {
        cvlr_assume!(pending_owner_internal != new_owner);
    }
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_ownership panics if the live_until_ledger is in the past.
// status: verified
pub fn transfer_ownership_panics_if_invalid_live_until_ledger(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    cvlr_assume!(live_until_ledger < e.ledger().sequence() || live_until_ledger > e.ledger().max_live_until_ledger());
    cvlr_assume!(live_until_ledger > 0);
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// accept_ownership panics if the not authorized by the pending owner.
// status: verified
pub fn accept_ownership_panics_if_unauth_by_pending_owner(e: Env) {
    let pending_owner = get_pending_owner(&e);
    if let Some(pending_owner_internal) = pending_owner.clone() {
        cvlr_assume!(!is_auth(pending_owner_internal));
    }
    OwnableContract::accept_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// accept_ownership panics if the pending owner is not set.
// status: verified
pub fn accept_ownership_panics_if_pending_owner_not_set(e: Env) {
    let pending_owner = get_pending_owner(&e);
    cvlr_assume!(pending_owner.is_none());
    OwnableContract::accept_ownership(&e);
    cvlr_assert!(false);
}

#[rule] 
// renounce_ownership panics if not authorized by the owner.
// status: verified
pub fn renounce_ownership_panics_if_unauth_by_owner(e: Env) {
    let owner = OwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
        cvlr_assume!(!is_auth(owner_internal));
    }
    OwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// renounce_ownership panics if the owner is not set.
// status: verified
pub fn renounce_ownership_panics_if_owner_not_set(e: Env) {
    let owner: Option<Address> = OwnableContract::get_owner(&e);
    cvlr_assume!(owner.is_none());
    OwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// renounce_ownership panics if there is a pending ownership transfer.
// status: verified
pub fn renounce_ownership_panics_if_pending_ownership_transfer(e: Env) {
    let pending_owner = e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner);
    cvlr_assume!(pending_owner.is_some());
    OwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}

// harness functions

#[rule]
// owner_restricted_function panics if not authorized by owner. 
// status: verified
pub fn owner_restricted_function_panics_if_unauth_by_owner(e: Env) {
    let owner = OwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
        cvlr_assume!(!is_auth(owner_internal));
    }
    OwnableContract::owner_restricted_function(&e);
    cvlr_assert!(false);
}

#[rule]
// owner_restricted_function panics if the owner is not set. 
// status: verified
pub fn owner_restricted_function_panics_if_owner_not_set(e: Env) {
    let owner = OwnableContract::get_owner(&e);
    cvlr_assume!(owner.is_none());
    OwnableContract::owner_restricted_function(&e);
    cvlr_assert!(false);
}
