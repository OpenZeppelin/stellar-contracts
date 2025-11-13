use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;

use soroban_sdk::{Env};

use crate::ownable::*;

#[rule]
pub fn set_owner_integrity(e: Env) {
    let new_owner = nondet_address();
    set_owner(&e, &new_owner);
    let owner_post = get_owner(&e);
    clog!(cvlr_soroban::Addr(&new_owner));
    if let Some(owner_post_internal) = owner_post.clone() {
        clog!(cvlr_soroban::Addr(&owner_post_internal));
    }
    cvlr_assert!(owner_post == Some(new_owner));
}

#[rule]
pub fn renounce_ownership_does_not_panic(e: Env) {
    use crate::ownable::storage::renounce_ownership;
    let setup = e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner);
    cvlr_assume!(setup.is_none());
    let res = renounce_ownership(&e);
    cvlr_assert!(true);
}