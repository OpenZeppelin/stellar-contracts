use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address,is_auth};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;

use soroban_sdk::{Env, Address};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

#[rule]
// transfer_ownership panics if the not authorized by the owner.
// status: 
pub fn transfer_ownership_panics_if_unauth_by_owner(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let owner = FVHarnessOwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
        cvlr_assume!(!is_auth(owner_internal));
    }
    FVHarnessOwnableContract::transfer_ownership(&e, new_owner.clone(), live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_ownership panics if the owner is not set.
// status: 
pub fn transfer_ownership_panics_if_owner_not_set(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assume!(owner == None);
    FVHarnessOwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// accept_ownership panics if the not authorized by the pending owner.
// status: 
pub fn accept_ownership_panics_if_unauth_by_pending_owner(e: Env) {
    let pending_owner_key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&pending_owner_key); // make sugar for this?
    if let Some(pending_owner_internal) = pending_owner.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
        cvlr_assume!(!is_auth(pending_owner_internal));
    }
    FVHarnessOwnableContract::accept_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// accept_ownership panics if the pending owner is not set.
// status: 
pub fn accept_ownership_panics_if_pending_owner_not_set(e: Env) {
    let pending_owner_key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&pending_owner_key);
    cvlr_assume!(pending_owner == None);
    FVHarnessOwnableContract::accept_ownership(&e);
    cvlr_assert!(false);
}

#[rule] 
// renounce_ownership panics if not authorized by the owner.
// status: 
pub fn renounce_ownership_panics_if_unauth_by_owner(e: Env) {
    let owner = FVHarnessOwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
        cvlr_assume!(!is_auth(owner_internal));
    }
    FVHarnessOwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// renounce_ownership panics if the owner is not set.
// status: 
pub fn renounce_ownership_panics_if_owner_not_set(e: Env) {
    let owner: Option<Address> = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assume!(owner == None);
    FVHarnessOwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// renounce_ownership panics if there is a pending ownership transfer.
// status: 
pub fn renounce_ownership_panics_if_pending_ownership_transfer(e: Env) {
    let pending_owner_key = OwnableStorageKey::PendingOwner;
    let pending_owner = e.storage().temporary().get::<_, Address>(&pending_owner_key);
    cvlr_assume!(pending_owner != None);
    FVHarnessOwnableContract::renounce_ownership(&e);
    cvlr_assert!(false);
}

#[rule]
// owner_restricted_function panics if not authorized by owner. 
// status: 
pub fn owner_restricted_function_panics_if_unauth_by_owner(e: Env) {
    let owner = FVHarnessOwnableContract::get_owner(&e);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
        cvlr_assume!(!is_auth(owner_internal));
    }
    FVHarnessOwnableContract::owner_restricted_function(&e);
    cvlr_assert!(false);
}

#[rule]
// owner_restricted_function panics if the owner is not set. 
// status: 
pub fn owner_restricted_function_panics_if_owner_not_set(e: Env) {
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assume!(owner == None);
    FVHarnessOwnableContract::owner_restricted_function(&e);
    cvlr_assert!(false);
}
