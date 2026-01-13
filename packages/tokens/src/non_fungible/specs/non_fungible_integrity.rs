use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{sequential, specs::helper::is_approved_for_token, Base};

#[rule]
// after transfer the token owner is set to the to address
// updates balances correctly
// status: verified
pub fn nft_transfer_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let balance_from_pre = Base::balance(&e, &from);
    clog!(balance_from_pre);
    let balance_to_pre = Base::balance(&e, &to);
    clog!(balance_to_pre);
    Base::transfer(&e, &from, &to, token_id);
    let owner_post = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(owner_post == to);
    let balance_from_post = Base::balance(&e, &from);
    clog!(balance_from_post);
    let balance_to_post = Base::balance(&e, &to);
    clog!(balance_to_post);
    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - 1);
        cvlr_assert!(balance_to_post == balance_to_pre + 1);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// after transfer_from the token owner is to
// updates balances correctly
// removes approval
// status: verified
pub fn nft_transfer_from_integrity(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let balance_from_pre = Base::balance(&e, &from);
    clog!(balance_from_pre);
    let balance_to_pre = Base::balance(&e, &to);
    clog!(balance_to_pre);
    Base::transfer_from(&e, &spender, &from, &to, token_id);
    let owner_post = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(owner_post == to);
    let balance_from_post = Base::balance(&e, &from);
    clog!(balance_from_post);
    let balance_to_post = Base::balance(&e, &to);
    clog!(balance_to_post);
    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - 1);
        cvlr_assert!(balance_to_post == balance_to_pre + 1);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
    let approval_post = Base::get_approved(&e, token_id);
    cvlr_assert!(approval_post.is_none());
}

#[rule]
// after approve the token owner is approved
// status: verified
pub fn nft_approve_integrity(e: Env) {
    let approver = nondet_address();
    clog!(cvlr_soroban::Addr(&approver));
    let approved = nondet_address();
    clog!(cvlr_soroban::Addr(&approved));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(live_until_ledger > 0);
    Base::approve(&e, &approver, &approved, token_id, live_until_ledger);
    let is_approved_for_token_post = is_approved_for_token(&e, &approver, &approved, token_id);
    clog!(is_approved_for_token_post);
    cvlr_assert!(is_approved_for_token_post);
}

#[rule]
// after approve_for_all the token owner is approved
// status: verified
pub fn nft_approve_for_all_integrity(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let token_id = u32::nondet(); // some token
    clog!(token_id);
    cvlr_assume!(live_until_ledger > 0);
    Base::approve_for_all(&e, &owner, &operator, live_until_ledger);
    let is_approved_for_token_post = is_approved_for_token(&e, &owner, &operator, token_id);
    clog!(is_approved_for_token_post);
    cvlr_assert!(is_approved_for_token_post);
}

#[rule]
// after mint the token owner is set to the to address
// updates balances correctly
// status: verified
pub fn nft_mint_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let balance_pre = Base::balance(&e, &to);
    clog!(balance_pre);
    Base::mint(&e, &to, token_id);
    let owner_post = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(owner_post == to);
    let balance_post = Base::balance(&e, &to);
    clog!(balance_post);
    cvlr_assert!(balance_post == balance_pre + 1);
}

#[rule]
// after sequential mint the token owner is set to the to address
// updates balances correctly
// status: verified
pub fn nft_sequential_mint_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let current_token_id = sequential::next_token_id(&e);
    clog!(current_token_id);
    let balance_pre = Base::balance(&e, &to);
    clog!(balance_pre);
    Base::sequential_mint(&e, &to);
    let owner_post = Base::owner_of(&e, current_token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(owner_post == to);
    let balance_post = Base::balance(&e, &to);
    clog!(balance_post);
    cvlr_assert!(balance_post == balance_pre + 1);
}
