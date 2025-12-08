use cvlr::{cvlr_satisfy, nondet::*, cvlr_assert, cvlr_assume};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};
use cvlr::clog;

use crate::non_fungible::Base;
use crate::non_fungible::storage::NFTStorageKey;

// invariant: token_owner -> balance >= 1 (can't do iff)

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

// helpers

pub fn assume_pre_owner_implies_balance(e: Env, token_id: u32) {
    clog!(token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    let balance_of_owner = Base::balance(&e, &owner);
    clog!(balance_of_owner);
    cvlr_assume!(balance_of_owner >= 1);
}

pub fn assert_post_owner_implies_balance(e: Env, token_id: u32) {
    clog!(token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    let balance_of_owner = Base::balance(&e, &owner);
    clog!(balance_of_owner);
    cvlr_assert!(balance_of_owner >= 1);
}

#[rule]
// status: spurious violation - prover doens't understand the relation between balance before and after
pub fn after_transfer_owner_implies_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let transferred_token_id = u32::nondet();
    clog!(transferred_token_id);
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_owner_implies_balance(e.clone(), token_id);
    Base::transfer(&e, &from, &to, transferred_token_id);
    assert_post_owner_implies_balance(e, token_id);
}

#[rule]
// status: spurious violation - prover doens't understand the relation between balance before and after
pub fn after_transfer_from_owner_implies_balance(e: Env) {
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
    assume_pre_owner_implies_balance(e.clone(), token_id);
    Base::transfer_from(&e, &spender, &from, &to, transferred_token_id);
    assert_post_owner_implies_balance(e, token_id);
}

#[rule]
// status: verified
pub fn after_approve_owner_implies_balance(e: Env) {
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
    assume_pre_owner_implies_balance(e.clone(), token_id);
    Base::approve(&e, &owner, &approved, approved_token_id, live_until_ledger);
    assert_post_owner_implies_balance(e, token_id);
}

#[rule]
// status: verified
pub fn after_approve_for_all_owner_implies_balance(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_owner_implies_balance(e.clone(), token_id);
    Base::approve_for_all(&e, &owner, &operator, live_until_ledger);
    assert_post_owner_implies_balance(e, token_id);
}

#[rule]
// status: verified
pub fn after_mint_owner_implies_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_owner_implies_balance(e.clone(), token_id);
    let minted_token_id = u32::nondet();
    clog!(minted_token_id);
    Base::mint(&e, &to, minted_token_id);
    assert_post_owner_implies_balance(e, token_id);
}