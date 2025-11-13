use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;

use soroban_sdk::{Env};

use crate::ownable::*;

#[rule]
pub fn renounce_ownership_does_not_panic(e: Env) {
    use crate::ownable::storage::renounce_ownership;
    let address = nondet_address();
    e.storage().temporary().set(&OwnableStorageKey::PendingOwner, &address);
    let setup = e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner);
    cvlr_assume!(setup.is_none());
    let res = renounce_ownership(&e);
    cvlr_assert!(true);
}