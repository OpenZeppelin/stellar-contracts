use cvlr::{cvlr_satisfy, nondet::*, cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address, is_auth};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};
use cvlr::clog;
use crate::non_fungible::Base;
use crate::non_fungible::specs::helper::is_approved_for_token;

#[rule]
// transfer_panics if not auth by from
// status: verified
pub fn nft_transfer_panics_if_unauthorized(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_auth(from.clone()));
    Base::transfer(&e, &from, &to, token_id);
    cvlr_assert!(false);
}

#[rule]
// transfer_panics if from doesn't own token
// status: verified
pub fn nft_transfer_panics_if_not_owner(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner != from);
    Base::transfer(&e, &from, &to, token_id);
    cvlr_assert!(false);
}


#[rule]
// transfer_from_panics if spender does not auth
// status: verified
pub fn nft_transfer_from_panics_if_unauthorized(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_auth(spender.clone()));
    Base::transfer_from(&e, &spender, &from, &to, token_id);
    cvlr_assert!(false);
}

#[rule]
// transfer_from_panics if from doesn't own token
// status: verified
pub fn nft_transfer_from_panics_if_not_owner(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner != from);
    Base::transfer_from(&e, &spender, &from, &to, token_id);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if is_approved_for_token returns false
// status: verified
pub fn nft_transfer_from_panics_if_not_approved(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_approved_for_token(&e, &from, &spender, token_id));
    Base::transfer_from(&e, &spender, &from, &to, token_id);
    cvlr_assert!(false);
}

#[rule]
// approve_panics if owner does not auth
// status: verified
pub fn nft_approve_panics_if_unauthorized(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let token_id = u32::nondet();
    clog!(token_id);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(!is_auth(owner.clone()));
    Base::approve(&e, &owner, &spender, token_id, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// approve_panics if live_until_ledger > max_ledger
// status: bug
// note: this is a low and also can be considered as a spurious violation (that is interesting to note)
pub fn nft_approve_panics_if_live_until_ledger_greater_than_max_ledger(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let token_id = u32::nondet();
    clog!(token_id);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(live_until_ledger > e.ledger().max_live_until_ledger());
    clog!(e.ledger().max_live_until_ledger());
    Base::approve(&e, &owner, &spender, token_id, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// approve_panics if live_until_ledger < current_ledger and non-zero
// status: verified
pub fn nft_approve_panics_if_live_until_ledger_less_than_current_ledger(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let token_id = u32::nondet();
    clog!(token_id);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let current_ledger = e.ledger().sequence();
    clog!(current_ledger);
    cvlr_assume!(live_until_ledger < current_ledger);
    cvlr_assume!(live_until_ledger > 0);
    Base::approve(&e, &owner, &spender, token_id, live_until_ledger);
    cvlr_assert!(false);
}



