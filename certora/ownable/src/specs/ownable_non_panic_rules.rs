use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;


use soroban_sdk::{Env, Address};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

#[rule]
pub fn renounce_ownership_does_not_panic(e: Env) {
    // use cvlr_soroban::require_storage_tag;
    
    // setup storage: needed for now. 
    let address = nondet_address();
    e.storage().temporary().set(&OwnableStorageKey::PendingOwner, &address);

    // WIP: will have this macro for setting storage up automatically.
    // require_storage_tag(OwnableStorageKey::PendingOwner.into_val(&e), 77);

    let setup = e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner);
    cvlr_assume!(setup.is_none());
    FVHarnessOwnableContract::renounce_ownership(&e);
    cvlr_assert!(true);
}