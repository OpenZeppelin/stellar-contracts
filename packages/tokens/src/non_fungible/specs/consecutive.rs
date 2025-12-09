use cvlr::{cvlr_satisfy, nondet::*, cvlr_assert, cvlr_assume};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};
use cvlr::clog;

use crate::non_fungible::extensions::consecutive::Consecutive;
use crate::non_fungible::sequential;
use crate::non_fungible::overrides::ContractOverrides;
use crate::non_fungible::specs::helper::is_approved_for_token;

// ################## INTEGRITY RULES ##################

// same integrity rules from non_fungible_integrity.rs 
// but the underlying functions are different.
// the code is very challenging for the prover
// need to think how we can simplify
// perhaps we can only analyze the internal functions such as update.

#[rule]
// after transfer the token owner is set to the to address
// updates balances correctly
// status: timeout
pub fn nft_consecutive_transfer_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Consecutive::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let balance_from_pre = Consecutive::balance(&e, &from);
    clog!(balance_from_pre);
    let balance_to_pre = Consecutive::balance(&e, &to);
    clog!(balance_to_pre);
    Consecutive::transfer(&e, &from, &to, token_id);
    let owner_post = Consecutive::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(owner_post == to);
    let balance_from_post = Consecutive::balance(&e, &from);
    clog!(balance_from_post);
    let balance_to_post = Consecutive::balance(&e, &to);
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
// status: timeout
pub fn nft_consecutive_transfer_from_integrity(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Consecutive::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let balance_from_pre = Consecutive::balance(&e, &from);
    clog!(balance_from_pre);
    let balance_to_pre = Consecutive::balance(&e, &to);
    clog!(balance_to_pre);
    Consecutive::transfer_from(&e, &spender, &from, &to, token_id);
    let owner_post = Consecutive::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(owner_post == to);
    let balance_from_post = Consecutive::balance(&e, &from);
    clog!(balance_from_post);
    let balance_to_post = Consecutive::balance(&e, &to);
    clog!(balance_to_post);
    if to != from {
    cvlr_assert!(balance_from_post == balance_from_pre - 1);
    cvlr_assert!(balance_to_post == balance_to_pre + 1);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
    let approval_post = Consecutive::get_approved(&e, token_id);
    cvlr_assert!(approval_post.is_none());
}

#[rule]
// after approve the token owner is approved
// status: verified 
// note: ±30 min and sanity unclear.
pub fn nft_consecutive_approve_integrity(e: Env) {
    let approver = nondet_address();
    clog!(cvlr_soroban::Addr(&approver));
    let approved = nondet_address();
    clog!(cvlr_soroban::Addr(&approved));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Consecutive::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(live_until_ledger > 0);
    Consecutive::approve(&e, &approver, &approved, token_id, live_until_ledger);
    let is_approved_for_token_post = is_approved_for_token(&e, &approver, &approved, token_id);
    clog!(is_approved_for_token_post);
    cvlr_assert!(is_approved_for_token_post);
}

// there is no approve_for_all function

#[rule]
// batch_mint changes balance correctly
// the owner_of the first_token id is "to"
// status: violation - unclear
pub fn nft_batch_mint_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount = u32::nondet();
    clog!(amount);
    let balance_pre = Consecutive::balance(&e, &to);
    clog!(balance_pre);
    let current_token_id = sequential::next_token_id(&e);
    clog!(current_token_id);
    Consecutive::batch_mint(&e, &to, amount);
    let balance_post = Consecutive::balance(&e, &to);
    clog!(balance_post);
    cvlr_assert!(balance_post == balance_pre + amount);
    let owner_of_current_token_id = Consecutive::owner_of(&e, current_token_id);
    clog!(cvlr_soroban::Addr(&owner_of_current_token_id));
    cvlr_assert!(owner_of_current_token_id == to);
}
