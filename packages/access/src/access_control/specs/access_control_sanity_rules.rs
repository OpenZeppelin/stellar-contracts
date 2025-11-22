
use cvlr::{cvlr_assert};
use cvlr_soroban::{nondet_address, nondet_symbol};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};

use crate::access_control::{AccessControl, specs::access_control_contract::AccessControlContract};

#[rule]
pub fn has_role_sanity(e: Env) {
    let role = nondet_symbol();
    let account = nondet_address();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::has_role(&e, account, role);
    cvlr_assert!(false);
}

#[rule]
pub fn get_admin_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::get_admin(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn get_role_member_count_sanity(e: Env) {
    let role = nondet_symbol();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    let _ = AccessControlContract::get_role_member_count(&e, role);
    cvlr_assert!(false);
}

#[rule]
pub fn get_role_member_sanity(e: Env) {
    let role = nondet_symbol();
    let i = u32::nondet();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    let _ = AccessControlContract::get_role_member(&e, role, i);
    cvlr_assert!(false);
}

#[rule]
pub fn get_role_admin_sanity(e: Env) {
    let role = nondet_symbol();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    let _ = AccessControlContract::get_role_admin(&e, role);
    cvlr_assert!(false);
}

#[rule]
pub fn set_admin_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    cvlr_assert!(false);
}

#[rule]
pub fn grant_role_sanity(e: Env) {
    let role = nondet_symbol();
    let caller = nondet_address();
    let account = nondet_address();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_assert!(false);
}

#[rule]
pub fn revoke_role_sanity(e: Env) {
    let role = nondet_symbol();
    let caller = nondet_address();
    let account = nondet_address();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_assert!(false);
}


#[rule]
pub fn renounce_role_sanity(e: Env) {
    let role = nondet_symbol();
    let caller = nondet_address();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_assert!(false);
}

#[rule]
pub fn transfer_admin_role_sanity(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
pub fn accept_admin_transfer_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn set_role_admin_sanity(e: Env) {
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::set_role_admin(&e, role, admin_role);
    cvlr_assert!(false);
}

#[rule]
pub fn renounce_admin_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::init(&e, admin);
    AccessControlContract::renounce_admin(&e);
    cvlr_assert!(false);
}