use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::Nondet};
use cvlr_soroban::{nondet_address, nondet_symbol};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Symbol};

use crate::access_control::{
    specs::{
        access_control_contract::AccessControlContract,
        constructor_helper::{
            before_constructor_no_has_role, before_constructor_no_role_accounts,
            before_constructor_no_role_count,
        },
        helper::{get_pending_admin, get_role_account},
    },
    AccessControl,
};

// invariant: admin != None -> holds in all cases except for renounce_admin

// helpers
pub fn assume_pre_admin_is_set(e: Env) {
    let admin_pre = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin_pre.is_some());
}

pub fn assert_post_admin_is_set(e: Env) {
    let admin_post = AccessControlContract::get_admin(&e);
    cvlr_assert!(admin_post.is_some());
}

#[rule]
// status: verified
pub fn after_constructor_admin_is_set(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_constructor_admin_is_set_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_grant_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_grant_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_revoke_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_revoke_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_renounce_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_renounce_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_set_role_admin_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_set_role_admin_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    cvlr_satisfy!(true);
}

// for the case renonuce_admin it's obviously true - and expected

// invariant: pending_admin != none implies admin != none

// helpers
pub fn assume_pre_pending_admin_implies_admin(e: &Env) {
    let pending_admin_pre = get_pending_admin(&e);
    let admin = AccessControlContract::get_admin(&e);
    if pending_admin_pre.is_some() {
        cvlr_assume!(admin.is_some());
    }
}

pub fn assert_post_pending_admin_implies_admin(e: &Env) {
    let pending_admin_post = get_pending_admin(&e);
    let admin = AccessControlContract::get_admin(&e);
    if pending_admin_post.is_some() {
        cvlr_assert!(admin.is_some());
    }
}

#[rule]
// status: verified
pub fn after_constructor_pending_admin_implies_admin(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_constructor_pending_admin_implies_admin_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_grant_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_grant_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_revoke_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_revoke_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_renounce_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_renounce_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_set_role_admin_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_set_role_admin_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    cvlr_satisfy!(true);
}

#[rule]
// status: bug
pub fn after_renounce_admin_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::renounce_admin(&e);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_renounce_admin_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::renounce_admin(&e);
    cvlr_satisfy!(true);
}