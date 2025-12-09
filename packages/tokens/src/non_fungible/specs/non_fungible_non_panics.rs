use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{is_auth, nondet_address};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    specs::{
        helper::is_approved_for_token, non_fungible_invariants::assume_pre_owner_implies_balance,
    },
    storage::{ApprovalData, NFTStorageKey},
    Base,
};

// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to
// consider also panicking paths.

// helpers - storage setup

pub fn storage_setup_owner(e: Env, token_id: u32) {
    let owner = nondet_address();
    e.storage().persistent().set(&NFTStorageKey::Owner(token_id), &owner);
}

pub fn storage_setup_balance(e: Env, account: Address) {
    let balance: u32 = nondet();
    e.storage().persistent().set(&NFTStorageKey::Balance(account), &balance);
    clog!(balance);
}

pub fn storage_setup_approved(e: Env, token_id: u32, approved: Address) {
    let live_until_ledger: u32 = nondet();
    clog!(token_id);
    clog!(cvlr_soroban::Addr(&approved));
    clog!(live_until_ledger);
    let approval = ApprovalData { approved: approved.clone(), live_until_ledger };
    e.storage().temporary().set(&NFTStorageKey::Approval(token_id), &approval);
}

pub fn storage_setup_approved_for_all(e: Env, owner: Address, operator: Address) {
    let live_until_ledger: u32 = nondet();
    clog!(cvlr_soroban::Addr(&owner));
    clog!(cvlr_soroban::Addr(&operator));
    clog!(live_until_ledger);
    e.storage()
        .temporary()
        .set(&NFTStorageKey::ApprovalForAll(owner.clone(), operator.clone()), &live_until_ledger);
}

pub fn reasonable_balance(e: Env, account: Address) {
    clog!(cvlr_soroban::Addr(&account));
    let balance = Base::balance(&e, &account);
    clog!(balance);
    cvlr_assume!(balance <= u32::MAX - 1);
}

// return to this after doing invariants.

#[rule]
// requires:
// - from is authenticated
// - owner == from (token owner matches from address)
// - to balance is reasonable (won't overflow)
// - pre-owner implies balance invariant holds
// storage setup:
// - owner for token_id
// - balance for from
// - balance for to
// status: verified
pub fn nft_transfer_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id: u32 = nondet();
    clog!(token_id);
    storage_setup_owner(e.clone(), token_id);
    storage_setup_balance(e.clone(), from.clone());
    storage_setup_balance(e.clone(), to.clone());
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(is_auth(from.clone()));
    cvlr_assume!(owner == from);
    reasonable_balance(e.clone(), to.clone());
    assume_pre_owner_implies_balance(e.clone(), token_id); // invariant proved in non_fungible_invariants.rs
    Base::transfer(&e, &from, &to, token_id);
    cvlr_assert!(true);
}

#[rule]
// requires:
// - owner == from (token owner matches from address)
// - spender is authenticated
// - spender is approved for token (via explicit approval, approval for all or
//   ownership)
// - to balance is reasonable (won't overflow)
// - pre-owner implies balance invariant holds
// storage setup:
// - owner for token_id
// - balance for from
// - balance for to
// - approval for token_id with spender
// - approval for all for from and spender
// status: verified
pub fn nft_transfer_from_non_panic(e: Env) {
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id: u32 = nondet();
    clog!(token_id);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    storage_setup_owner(e.clone(), token_id);
    storage_setup_balance(e.clone(), from.clone());
    storage_setup_balance(e.clone(), to.clone());
    storage_setup_approved(e.clone(), token_id, spender.clone());
    storage_setup_approved_for_all(e.clone(), from.clone(), spender.clone());
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner == from);
    cvlr_assume!(is_auth(spender.clone()));
    cvlr_assume!(is_approved_for_token(&e, &from, &spender, token_id));
    reasonable_balance(e.clone(), to.clone());
    assume_pre_owner_implies_balance(e.clone(), token_id); // invariant proved in non_fungible_invariants.rs
    Base::transfer_from(&e, &spender, &from, &to, token_id);
    cvlr_assert!(true);
}

#[rule]
// requires:
// - approver is authenticated
// - approver is owner OR approver is approved for all by owner
// - live_until_ledger is 0 OR (live_until_ledger <= max_live_until_ledger AND
//   live_until_ledger > current_ledger)
// storage setup:
// - owner for token_id
// - approval_for_all for owner and approver
// status: verified
pub fn nft_approve_non_panic(e: Env) {
    let approver = nondet_address();
    clog!(cvlr_soroban::Addr(&approver));
    let approved = nondet_address();
    clog!(cvlr_soroban::Addr(&approved));
    let token_id: u32 = nondet();
    clog!(token_id);
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    storage_setup_owner(e.clone(), token_id);
    let owner = Base::owner_of(&e, token_id);
    let current_ledger = e.ledger().sequence();
    let max_live_until_ledger = e.ledger().max_live_until_ledger();
    let ledger_leq_max = live_until_ledger <= max_live_until_ledger;
    let ledger_above_current = live_until_ledger > current_ledger;
    let live_until_ledger_is_zero = live_until_ledger == 0;
    cvlr_assume!(is_auth(approver.clone()));
    let owner_is_approver = owner == approver;
    storage_setup_approved_for_all(e.clone(), owner.clone(), approver.clone());
    let approver_is_approved_autherator = Base::is_approved_for_all(&e, &owner, &approver);
    cvlr_assume!(owner_is_approver || approver_is_approved_autherator);
    cvlr_assume!(live_until_ledger_is_zero || (ledger_leq_max && ledger_above_current));
    Base::approve(&e, &approver, &approved, token_id, live_until_ledger);
    cvlr_assert!(true);
}

#[rule]
// requires:
// - owner is authenticated
// - live_until_ledger is 0 OR (live_until_ledger <= max_live_until_ledger AND
//   live_until_ledger > current_ledger)
// status: verified
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
    let ledger_above_current = live_until_ledger > current_ledger;
    let live_until_ledger_is_zero = live_until_ledger == 0;
    cvlr_assume!(is_auth(owner.clone()));
    cvlr_assume!(live_until_ledger_is_zero || (ledger_leq_max && ledger_above_current));
    Base::approve_for_all(&e, &owner, &operator, live_until_ledger);
    cvlr_assert!(true);
}
