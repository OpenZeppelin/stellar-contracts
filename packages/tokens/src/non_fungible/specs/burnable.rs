use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{is_auth, nondet_address};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    specs::helper::{is_approved_for_token, is_owned},
    Base,
};

// ################## INTEGRITY RULES ##################

#[rule]
// after burn the account's balance decreases by 1
// and the token has no owner
// status: verified
pub fn nft_burn_integrity(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let balance_pre = Base::balance(&e, &from);
    clog!(balance_pre);
    let owner_pre = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    Base::burn(&e, &from, token_id);
    let balance_post = Base::balance(&e, &from);
    clog!(balance_post);
    let owner_post = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(balance_post == balance_pre - 1);
    cvlr_assert!(!is_owned(&e, token_id));
}

#[rule]
// after burn_from the account's balance decreases by 1
// and the token has no owner
// status: verified
pub fn nft_burn_from_integrity(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let balance_pre = Base::balance(&e, &from);
    clog!(balance_pre);
    let owner_pre = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    Base::burn_from(&e, &spender, &from, token_id);
    let balance_post = Base::balance(&e, &from);
    clog!(balance_post);
    let owner_post = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_post));
    cvlr_assert!(balance_post == balance_pre - 1);
    cvlr_assert!(!is_owned(&e, token_id));
}

// ################## PANIC RULES ##################

#[rule]
// burn panics if not auth by from
// status: verified
pub fn nft_burn_panics_if_unauthorized(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_auth(from.clone()));
    Base::burn(&e, &from, token_id);
    cvlr_assert!(false);
}

#[rule]
// burn panics if from is not the token owner
// status: verified
pub fn nft_burn_panics_wrong_owner(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner != from);
    Base::burn(&e, &from, token_id);
    cvlr_assert!(false);
}

#[rule]
// burn panics if token is not owned
// status: verified
pub fn nft_burn_panics_if_not_owned(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_owned(&e, token_id));
    Base::burn(&e, &from, token_id);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if not auth by spender
// status: verified
pub fn nft_burn_from_panics_if_unauthorized(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_auth(spender.clone()));
    Base::burn_from(&e, &spender, &from, token_id);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if from is not the token owner
// status: verified
pub fn nft_burn_from_panics_wrong_owner(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner = Base::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(owner != from);
    Base::burn_from(&e, &spender, &from, token_id);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if token is not owned
// status: verified
pub fn nft_burn_from_panics_if_not_owned(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_owned(&e, token_id));
    Base::burn_from(&e, &spender, &from, token_id);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if not approved
// status: verified
pub fn nft_burn_from_panics_if_not_approved(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    cvlr_assume!(!is_approved_for_token(&e, &from, &spender, token_id));
    Base::burn_from(&e, &spender, &from, token_id);
    cvlr_assert!(false);
}
