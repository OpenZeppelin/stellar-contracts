use cvlr::{cvlr_assert};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::clog;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env,Address};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

#[rule]
// after the constructor the owner is set.
// status: verified
pub fn constructor_integrity(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));

    FVHarnessOwnableContract::__constructor(&e, new_owner.clone());
    let owner_post = FVHarnessOwnableContract::get_owner(&e);
    
    if let Some(owner_post_internal) = owner_post.clone() {
        clog!(cvlr_soroban::Addr(&owner_post_internal));
    }
    cvlr_assert!(owner_post == Some(new_owner));
}

#[rule]
// transfer_ownership with a live ledger above current sets the pending owner to input.
// status: verified
pub fn transfer_ownership_integrity(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let current_ledger = e.ledger().sequence();
    clog!(current_ledger);
    cvlr_assume!(live_until_ledger > current_ledger); // assuming a proper transfer

    FVHarnessOwnableContract::transfer_ownership(&e, new_owner.clone(), live_until_ledger);
    
    let pending_owner_key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&pending_owner_key); // make sugar for this?
    if let Some(pending_owner_internal) = pending_owner.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }

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

    FVHarnessOwnableContract::transfer_ownership(&e, new_owner.clone(), live_until_ledger);

    let pending_owner_key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&pending_owner_key);
    cvlr_assert!(pending_owner == None);
}

#[rule]
// accept_ownership sets the owner to the pending owner and removes the pending owner.
// status: verified
pub fn accept_ownership_integrity(e: Env) {

    let pending_owner_key = OwnableStorageKey::PendingOwner;
    let pending_owner_pre = e.storage().temporary().get::<_, Address>(&pending_owner_key);
    if let Some(pending_owner_internal) = pending_owner_pre.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }

    FVHarnessOwnableContract::accept_ownership(&e);

    let owner = FVHarnessOwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
    }

    cvlr_assert!(owner == pending_owner_pre);
    cvlr_assert!(owner != None);

    let pending_owner_post = e.storage().temporary().get::<_, Address>(&pending_owner_key);
    if let Some(pending_owner_internal) = pending_owner_post.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }
    cvlr_assert!(pending_owner_post == None);
}

#[rule]
// renounce_ownership removes the owner.
// status: verified
pub fn renounce_ownership_integrity(e: Env) {

    FVHarnessOwnableContract::renounce_ownership(&e);

    let owner = FVHarnessOwnableContract::get_owner(&e);

    cvlr_assert!(owner == None);
}