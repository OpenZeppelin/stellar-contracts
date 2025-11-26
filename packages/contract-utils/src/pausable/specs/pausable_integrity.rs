use cvlr::{cvlr_assert,cvlr_satisfy};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use cvlr::clog;
use soroban_sdk::Env;
use crate::pausable::specs::pausable_contract::PausableContract;
use crate::pausable::Pausable;  
use crate::pausable::{pause, paused};

#[rule]
// after call to pause the contract is paused
// status: verified
pub fn pause_integrity(e: Env) {
    let caller = nondet_address();
    PausableContract::pause(&e, caller);
    let paused_post = PausableContract::paused(&e);
    cvlr_assert!(paused_post);
}

#[rule]
// after call to unpause the contract is not paused
// status: verified
pub fn unpause_integrity(e: Env) {
    let caller = nondet_address();
    PausableContract::unpause(&e, caller);
    let paused_post = PausableContract::paused(&e);
    cvlr_assert!(!paused_post);
}