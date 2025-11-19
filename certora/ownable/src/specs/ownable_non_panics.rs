use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address, is_auth};
use cvlr_soroban_derive::rule;


use soroban_sdk::{Env};

use stellar_access::ownable::*;

use crate::ownable_contract::OwnableContract;
use crate::specs::helper::get_pending_owner;
// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to consider also panicking paths.

// TODO:
// non-panic for transfer_ownership
// non-panic for accept_ownership

#[rule]
// requires
// pending_owner is an address
// pending_owner is none
// 
// status: 
pub fn renounce_ownership_non_panic(e: Env) {
    // use cvlr_soroban::require_storage_tag;
    
    // setup storage: needed for now. 
    let address = nondet_address();
    e.storage().temporary().set(&OwnableStorageKey::PendingOwner, &address);

    // WIP: will have this macro for setting storage up automatically.
    // require_storage_tag(OwnableStorageKey::PendingOwner.into_val(&e), 77);

    let pending_owner = get_pending_owner(&e);
    cvlr_assume!(pending_owner.is_none());
    let owner = OwnableContract::get_owner(&e);
    cvlr_assume!(owner.is_some());
    if let Some(owner_internal) = owner.clone() {
        cvlr_assume!(is_auth(owner_internal));
    }
    
    OwnableContract::renounce_ownership(&e);
    cvlr_assert!(true);
}

// non-panic for owner_restricted_function