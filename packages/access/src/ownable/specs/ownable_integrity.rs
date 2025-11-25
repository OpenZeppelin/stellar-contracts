use cvlr::{cvlr_assert, cvlr_assume,cvlr_satisfy};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::clog;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};

use crate::ownable::{specs::{helper::get_pending_owner, ownable_contract::OwnableContract}, *};

#[rule]
// after the constructor the owner is set.
// status: verified
pub fn ownable_constructor_integrity(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));

    OwnableContract::ownable_constructor(&e, new_owner.clone());
    let owner_post = OwnableContract::get_owner(&e);
    
    if let Some(owner_post_internal) = owner_post.clone() {
        clog!(cvlr_soroban::Addr(&owner_post_internal));
    }
    cvlr_assert!(owner_post == Some(new_owner));
}

#[rule]
// transfer_ownership with live_until_ledger > current_ledger
// sets the pending owner to new_owner and does not change the owner
// status: verified
pub fn transfer_ownership_integrity(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let current_ledger = e.ledger().sequence();
    clog!(current_ledger);
    cvlr_assume!(live_until_ledger > current_ledger); // assuming a proper transfer
    let owner_pre = OwnableContract::get_owner(&e);
    if let Some(owner_pre_internal) = owner_pre.clone() {
        clog!(cvlr_soroban::Addr(&owner_pre_internal));
    }
    OwnableContract::transfer_ownership(&e, new_owner.clone(), live_until_ledger);
    
    let pending_owner = get_pending_owner(&e);
    if let Some(pending_owner_internal) = pending_owner.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }
    let owner_post = OwnableContract::get_owner(&e);
    if let Some(owner_post_internal) = owner_post.clone() {
        clog!(cvlr_soroban::Addr(&owner_post_internal));
    }
    cvlr_assert!(owner_post == owner_pre);
    cvlr_assert!(pending_owner == Some(new_owner));
    // TODO : assert about TTL.
}


#[rule]
// transfer_ownership with a live ledger 0 removes the pending owner.
// status: verified
pub fn remove_transfer_ownership_integrity(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));
    let live_until_ledger = 0;
    clog!(live_until_ledger);
    let current_ledger = e.ledger().sequence();
    clog!(current_ledger);

    OwnableContract::transfer_ownership(&e, new_owner.clone(), live_until_ledger);

    let pending_owner = get_pending_owner(&e);
    cvlr_assert!(pending_owner.is_none());
}

#[rule]
// accept_ownership sets the owner to the pending owner and removes the pending owner.
// status: verified
pub fn accept_ownership_integrity(e: Env) {

    let pending_owner_pre = get_pending_owner(&e);
    if let Some(pending_owner_internal) = pending_owner_pre.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }

    OwnableContract::accept_ownership(&e);

    let owner = OwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
    }

    cvlr_assert!(owner == pending_owner_pre);
    cvlr_assert!(!owner.is_none());

    let pending_owner_post = get_pending_owner(&e);
    if let Some(pending_owner_internal) = pending_owner_post.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }
    cvlr_assert!(pending_owner_post.is_none());
}

#[rule]
// renounce_ownership removes the owner.
// status: verified
pub fn renounce_ownership_integrity(e: Env) {

    OwnableContract::renounce_ownership(&e);

    let owner = OwnableContract::get_owner(&e);

    cvlr_assert!(owner.is_none());
}