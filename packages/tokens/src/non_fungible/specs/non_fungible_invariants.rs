use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{storage::NFTStorageKey, Base};

// invariant: token owner exists after a mint forever

// helpers

pub fn assume_pre_token_owner_exists(e: Env, token_id: u32) {
    clog!(token_id);
    let key = NFTStorageKey::Owner(token_id);
    let owner = e.storage().persistent().get::<_, Address>(&key);
    cvlr_assume!(owner.is_some());
    if let Some(owner) = owner {
        clog!(cvlr_soroban::Addr(&owner));
    }
}

pub fn assert_post_token_owner_exists(e: Env, token_id: u32) {
    clog!(token_id);
    let key = NFTStorageKey::Owner(token_id);
    let owner = e.storage().persistent().get::<_, Address>(&key);
    cvlr_assert!(owner.is_some());
    if let Some(owner) = owner {
        clog!(cvlr_soroban::Addr(&owner));
    }
}

#[rule]
// status: verified
pub fn after_transfer_token_owner_exists(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let transferred_token_id = u32::nondet();
    clog!(transferred_token_id);
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_token_owner_exists(e.clone(), token_id);
    Base::transfer(&e, &from, &to, transferred_token_id);
    assert_post_token_owner_exists(e, token_id);
}

#[rule]
// status: verified
pub fn after_transfer_from_token_owner_exists(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let transferred_token_id = u32::nondet();
    clog!(transferred_token_id);
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_token_owner_exists(e.clone(), token_id);
    Base::transfer_from(&e, &spender, &from, &to, transferred_token_id);
    assert_post_token_owner_exists(e, token_id);
}

#[rule]
// status: verified
pub fn after_approve_token_owner_exists(e: Env) {
    let approved = nondet_address();
    clog!(cvlr_soroban::Addr(&approved));
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let token_id = u32::nondet();
    clog!(token_id);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let approved_token_id = u32::nondet();
    clog!(approved_token_id);
    assume_pre_token_owner_exists(e.clone(), token_id);
    Base::approve(&e, &owner, &approved, approved_token_id, live_until_ledger);
    assert_post_token_owner_exists(e, token_id);
}

#[rule]
// status: verified
pub fn after_approve_for_all_token_owner_exists(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_token_owner_exists(e.clone(), token_id);
    Base::approve_for_all(&e, &owner, &operator, live_until_ledger);
    assert_post_token_owner_exists(e, token_id);
}

#[rule]
// this is "init" for this invariant.
// status: verified
pub fn after_mint_token_owner_exists(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    Base::mint(&e, &to, token_id);
    assert_post_token_owner_exists(e, token_id);
}

// we would like to do owner(of) = account => balance(account) >= 1 
// but this doesnt work without ghosts and hooks -- similar issue to fungible.