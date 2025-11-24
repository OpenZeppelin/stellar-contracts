use cvlr::{cvlr_assert,cvlr_satisfy};use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};

use crate::ownable::*;

use crate::ownable::specs::ownable_contract::OwnableContract;

#[rule]
pub fn get_owner_sanity(e: Env) {
    let owner = nondet_address();
    OwnableContract::ownable_constructor(&e, owner);
    let _ =OwnableContract:: get_owner(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_owner_sanity(e: Env) {
    let owner = nondet_address();
    OwnableContract::ownable_constructor(&e, owner);
    cvlr_satisfy!(true);
}

#[rule]
pub fn transfer_ownership_sanity(e: Env) {
    let owner = nondet_address();
    OwnableContract::ownable_constructor(&e, owner);
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    OwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn accept_ownership_sanity(e: Env) {
    let owner = nondet_address();
    OwnableContract::ownable_constructor(&e, owner);
    OwnableContract::accept_ownership(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn renounce_ownership_sanity(e: Env) {
    let owner = nondet_address();
    OwnableContract::ownable_constructor(&e, owner);
    OwnableContract::renounce_ownership(&e);
    cvlr_satisfy!(true);
}