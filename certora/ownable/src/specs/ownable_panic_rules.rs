use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy};
use cvlr_soroban::{nondet_address,is_auth};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;

use soroban_sdk::{Env, Address};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

#[rule]
// status: issue "unreachable"
pub fn transfer_ownership_panics_if_unauth(e: Env) {
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
// status: issue "unreachable"
pub fn transfer_ownship_panics_if_owner_not_set(e: Env) {
    let new_owner = nondet_address();
    let live_until_ledger = u32::nondet();
    let owner = FVHarnessOwnableContract::get_owner(&e);
    cvlr_assert!(owner == None);
    FVHarnessOwnableContract::transfer_ownership(&e, new_owner, live_until_ledger);
    cvlr_assert!(false);
}