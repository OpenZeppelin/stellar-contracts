use cvlr::{cvlr_assert,cvlr_assume,cvlr_satisfy};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;
use crate::pausable::specs::pausable_contract::PausableContract;
use crate::pausable::Pausable;  
use crate::pausable::{pause, paused};


#[rule]
// pause panics if the contract is already paused
// status: verified
pub fn pause_panics_if_paused(e: Env) {
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(paused_pre);
    let caller = nondet_address();
    PausableContract::pause(&e, caller);
    cvlr_assert!(false);
}

#[rule]
// unpause panics if the contract is not paused
// status: verified with precise_bitwise_ops
pub fn unpause_panics_if_not_paused(e: Env) {
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(!paused_pre);
    let caller = nondet_address();
    PausableContract::unpause(&e, caller);
    cvlr_assert!(false);
}

#[rule]
// when_not_paused_func panics if paused
// status: verified
pub fn when_not_paused_panics_if_paused(e: Env) {
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(paused_pre);
    PausableContract::when_not_paused_func(&e);
    cvlr_assert!(false);
}

#[rule]
// when_paused_func panics if not paused
// status: verified with precise_bitwise_ops
pub fn when_paused_panics_if_not_paused(e: Env) {
    let paused_pre = PausableContract::paused(&e);
    cvlr_assume!(!paused_pre);
    PausableContract::when_paused_func(&e);
    cvlr_assert!(false);
}