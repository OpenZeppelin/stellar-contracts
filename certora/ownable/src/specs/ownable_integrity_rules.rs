use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::clog;

use soroban_sdk::{Env};

use stellar_access::ownable::*;

use crate::ownable_contract::FVHarnessOwnableContract;

#[rule]
pub fn set_owner_integrity(e: Env) {
    let new_owner = nondet_address();
    clog!(cvlr_soroban::Addr(&new_owner));

    let owner_post = FVHarnessOwnableContract::get_owner(&e);
    
    if let Some(owner_post_internal) = owner_post.clone() {
        clog!(cvlr_soroban::Addr(&owner_post_internal));
    }
    cvlr_assert!(owner_post == Some(new_owner));
}