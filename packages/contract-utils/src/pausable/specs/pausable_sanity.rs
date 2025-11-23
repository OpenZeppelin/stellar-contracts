use cvlr::{cvlr_assert,cvlr_satisfy};use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;

use soroban_sdk::{Env};

use crate::pausable::{specs::pausable_contract::PausableContract, *};

#[rule]
pub fn paused_sanity(e: Env) {
    PausableContract::paused(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn pause_sanity(e: Env) {
    let caller = nondet_address();
    PausableContract::pause(&e, caller);
    cvlr_satisfy!(true);
}

#[rule]
pub fn unpause_sanity(e: Env) {
    let caller = nondet_address();
    PausableContract::unpause(&e, caller);
    cvlr_satisfy!(true);
}

#[rule]
pub fn when_not_paused_sanity(e: Env) {
    when_not_paused(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn when_paused_sanity(e: Env) {
    when_paused(&e);
    cvlr_satisfy!(true);
}