use cvlr::{cvlr_satisfy, nondet::*, cvlr_assert, cvlr_assume};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};
use cvlr::clog;
use crate::non_fungible::Base;
use crate::non_fungible::specs::helper::is_approved_for_token;
use cvlr_soroban::is_auth;
use crate::non_fungible::storage::NFTStorageKey;

// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to consider also panicking paths.

// helpers - storage setup

pub fn storage_setup_owner(e: Env, token_id: u32) {
    let owner = nondet_address();
    e.storage().persistent().set(&NFTStorageKey::Owner(token_id), &owner);
}

// return to this after doing invariants.

#[rule]
// requires
// owner = from
// from auth
// status: violated - see decrease_balance
pub fn nft_transfer_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id: u32 = nondet();
    clog!(token_id);
    cvlr_assume!(is_auth(from.clone()));
    storage_setup_owner(e.clone(), token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner == from);
    Base::transfer(&e, &from, &to, token_id);
    cvlr_assert!(true);
}

#[rule]
// requires
// owner = from
// spender auth
// spender is approved
// status: violated - storage?
pub fn nft_transfer_from_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id: u32 = nondet();
    clog!(token_id);
    storage_setup_owner(e.clone(), token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner == from);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    cvlr_assume!(is_auth(spender.clone()));
    cvlr_assume!(is_approved_for_token(&e, &from, &spender, token_id));
    Base::transfer_from(&e, &spender, &from, &to, token_id);
    cvlr_assert!(true);
}

#[rule]
// requires
// owner auth
// live until ledger is appropriate
// status: violated - storage
pub fn nft_approve_non_panic(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let token_id: u32 = nondet();
    clog!(token_id);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let current_ledger = e.ledger().sequence();
    let max_live_until_ledger = e.ledger().max_live_until_ledger();
    let ledger_leq_max = live_until_ledger <= max_live_until_ledger;
    let ledger_above_currnet = live_until_ledger > current_ledger;
    let live_until_ledger_is_zero = live_until_ledger == 0;
    cvlr_assume!(live_until_ledger_is_zero || (ledger_leq_max && ledger_above_currnet));
    cvlr_assume!(is_auth(owner.clone()));
    Base::approve(&e, &owner, &spender, token_id, live_until_ledger);
    cvlr_assert!(true);
}

#[rule]
// requires
// owner auth
// live until ledger is appropriate
// status:
pub fn nft_approve_for_all_non_panic(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let current_ledger = e.ledger().sequence();
    let max_live_until_ledger = e.ledger().max_live_until_ledger();
    let ledger_leq_max = live_until_ledger <= max_live_until_ledger;
    let ledger_above_currnet = live_until_ledger > current_ledger;
    let live_until_ledger_is_zero = live_until_ledger == 0;
    cvlr_assume!(live_until_ledger_is_zero || (ledger_leq_max && ledger_above_currnet));
    cvlr_assume!(is_auth(owner.clone()));
    Base::approve_for_all(&e, &owner, &operator, live_until_ledger);
    cvlr_assert!(true);
}