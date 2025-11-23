use cvlr::{cvlr_assert,cvlr_assume,cvlr_satisfy};
use cvlr_soroban::nondet_address;
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;
use crate::pausable::specs::pausable_contract::PausableContract;
use crate::pausable::Pausable;  
use crate::pausable::{pause, paused};
use crate::pausable::storage::PausableStorageKey;

// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to consider also panicking paths.

#[rule]
// requires
// unpaused
// status: verified
pub fn pause_non_panic(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(!paused_pre);
    let caller = nondet_address();
    PausableContract::pause(&e, caller);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn pause_non_panic_sanity(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(!paused_pre);
    let caller = nondet_address();
    PausableContract::pause(&e, caller);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// paused
// status: verified
pub fn unpause_non_panic(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(paused_pre);
    let caller = nondet_address();
    PausableContract::unpause(&e, caller);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn unpause_non_panic_sanity(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(paused_pre);
    let caller = nondet_address();
    PausableContract::unpause(&e, caller);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// unpaused
// status: verified
pub fn when_not_paused_non_panic(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(!paused_pre);
    PausableContract::when_not_paused_func(&e);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn when_not_paused_non_panic_sanity(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(!paused_pre);
    PausableContract::when_not_paused_func(&e);
    cvlr_satisfy!(true);
}
#[rule]
// requires
// paused
// status: verified
pub fn when_paused_non_panic(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(paused_pre);
    PausableContract::when_paused_func(&e);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn when_paused_non_panic_sanity(e: Env) {
    // storage set up
    let bool = bool::nondet();    
    e.storage().instance().set(&PausableStorageKey::Paused, &bool);
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(paused_pre);
    PausableContract::when_paused_func(&e);
    cvlr_satisfy!(true);
}