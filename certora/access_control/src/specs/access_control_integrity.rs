use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban::{nondet_address, nondet_symbol};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;
use cvlr::clog;

use soroban_sdk::{Env};

use crate::access_control_contract::AccessControlContract;
use stellar_access::access_control::AccessControl;
use crate::specs::helper::get_pending_admin;

#[rule]
// after call to constructor the admin is set
// status: failing - spurious
pub fn constructor_integrity(e: Env) {
    let admin = nondet_address();
    clog!(cvlr_soroban::Addr(&admin));
    AccessControlContract::__constructor(&e, admin.clone());
    let admin_post = AccessControlContract::get_admin(&e);
    if let Some(admin_post_internal) = admin_post.clone() {
        clog!(cvlr_soroban::Addr(&admin_post_internal));
    }
    cvlr_assert!(admin_post == Some(admin.clone()));
}

#[rule]
// after call to grant_role the account has the role
// status: failing - spurious
pub fn grant_role_integrity(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller.clone(), account.clone(), role.clone());
    let account_has_role = AccessControlContract::has_role(&e, account.clone(), role.clone());
    cvlr_assert!(account_has_role != None);
}

#[rule]
// after call to revoke_role the account does not have the role
// status: failing - spurious
pub fn revoke_role_integrity(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller.clone(), account.clone(), role.clone());
    let account_has_role = AccessControlContract::has_role(&e, account.clone(), role.clone());
    cvlr_assert!(account_has_role == None);
}

#[rule]
// after call to renounce_role the account does not have the role
// status: failing - spurious
pub fn renounce_role_integrity(e: Env) {
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller.clone(), role.clone());
    let account_has_role = AccessControlContract::has_role(&e, caller.clone(), role.clone());
    cvlr_assert!(account_has_role == None);
}

#[rule]
// after call to transfer_admin_role with live_until_ledger > current_ledger the pending admin is set to the new admin
// status: 
pub fn transfer_admin_role_integrity(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    let current_ledger = e.ledger().sequence();
    cvlr_assume!(live_until_ledger > current_ledger); // proper admin transfer
    AccessControlContract::transfer_admin_role(&e, new_admin.clone(), live_until_ledger);
    let pending_admin = get_pending_admin(&e);
    cvlr_assert!(pending_admin == Some(new_admin.clone()));
}

#[rule]
// after call to accept_admin_transfer with live_until_ledger = 0 the pending admin is none
// status:
pub fn remove_transfer_admin_role_integrity(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = 0;
    AccessControlContract::transfer_admin_role(&e, new_admin.clone(), live_until_ledger);
    let pending_admin = get_pending_admin(&e);
    cvlr_assert!(pending_admin == None);
}

#[rule]
// after call to accept_admin_transfer the admin is set to the previous pending admin, which is not none, and the pending admin is set to none
// status:
pub fn accept_admin_transfer_integrity(e: Env) {
    let pending_admin_pre = get_pending_admin(&e);
    AccessControlContract::accept_admin_transfer(&e);
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin.clone() {
        clog!(cvlr_soroban::Addr(&admin_internal));
    }
    cvlr_assert!(admin == pending_admin_pre);
    cvlr_assert!(admin != None);
    let pending_admin_post   = get_pending_admin(&e);
    cvlr_assert!(pending_admin_post == None);
}

#[rule]
// after call to set_role_admin the role admin of the given role is the given admin_role
// status:
pub fn set_role_admin_integrity(e: Env) {
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role.clone(), admin_role.clone());
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    cvlr_assert!(role_admin == Some(admin_role.clone()));
}

#[rule]
// after call to renounce_admin the admin is none
// status:
pub fn renounce_admin_integrity(e: Env) {
    AccessControlContract::renounce_admin(&e);
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assert!(admin == None);
}