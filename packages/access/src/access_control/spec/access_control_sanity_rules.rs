
use cvlr::{cvlr_assert};
use cvlr_soroban::{nondet_address};
use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env, Symbol};

use crate::access_control::*;

// TODO: need nondet for Symbol
// until then pass it as argument

#[rule]
pub fn has_role_sanity(e: Env, role: Symbol) {
    let account = nondet_address();
    has_role(&e, &account, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn get_admin_sanity(e: Env) {
    get_admin(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn get_role_member_count_sanity(e: Env, role: Symbol) {
    let _ = get_role_member_count(&e, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn get_role_member_sanity(e: Env, role: Symbol) {
    let i = u32::nondet();
    let _ = get_role_member(&e, &role, i);
    cvlr_assert!(false);
}

#[rule]
pub fn get_role_admin_sanity(e: Env, role: Symbol) {
    let _ = get_role_admin(&e, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn set_admin_sanity(e: Env) {
    let admin = nondet_address();
    set_admin(&e, &admin);
    cvlr_assert!(false);
}

#[rule]
pub fn grant_role_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    let account = nondet_address();
    grant_role(&e, &caller, &account, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn grant_role_no_auth_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    let account = nondet_address();
    grant_role_no_auth(&e, &caller, &account, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn revoke_role_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    let account = nondet_address();
    revoke_role(&e, &caller, &account, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn revoke_role_no_auth_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    let account = nondet_address();
    revoke_role_no_auth(&e, &caller, &account, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn renounce_role_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    renounce_role(&e, &caller, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn transfer_admin_role_sanity(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    transfer_admin_role(&e, &new_admin, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
pub fn accept_admin_transfer_sanity(e: Env) {
    accept_admin_transfer(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn set_role_admin_sanity(e: Env, role: Symbol, admin_role: Symbol) {
    set_role_admin(&e, &role, &admin_role);
    cvlr_assert!(false);
}

#[rule]
pub fn renounce_admin_sanity(e: Env) {
    renounce_admin(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn set_role_admin_no_auth_sanity(e: Env, role: Symbol, admin_role: Symbol) {
    set_role_admin_no_auth(&e, &role, &admin_role);
    cvlr_assert!(false);
}

#[rule]
pub fn remove_role_admin_no_auth_sanity(e: Env, role: Symbol) {
    remove_role_admin_no_auth(&e, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn remove_role_accounts_count_no_auth_sanity(e: Env, role: Symbol) {
    remove_role_accounts_count_no_auth(&e, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn ensure_if_admin_or_admin_role_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    ensure_if_admin_or_admin_role(&e, &caller, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn ensure_role_sanity(e: Env, role: Symbol) {
    let caller = nondet_address();
    ensure_role(&e, &caller, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn enforce_admin_auth_sanity(e: Env) {
    let _ = enforce_admin_auth(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn add_to_role_enumeration_sanity(e: Env, role: &Symbol) {
    let account = nondet_address();
    add_to_role_enumeration(&e, &account, &role);
    cvlr_assert!(false);
}

#[rule]
pub fn remove_from_role_enumeration_sanity(e: Env, role: &Symbol) {
    let account = nondet_address();
    remove_from_role_enumeration(&e, &account, &role);
    cvlr_assert!(false);
}