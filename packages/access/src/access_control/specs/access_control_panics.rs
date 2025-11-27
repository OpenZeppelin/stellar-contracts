
use cvlr::{cvlr_assert, cvlr_assume,cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_symbol, is_auth};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;
use cvlr::clog;

use soroban_sdk::{Env};
use crate::access_control::{AccessControl, ensure_role, specs::{access_control_contract::AccessControlContract, helper::get_pending_admin}};


// package functions

#[rule]
// grant role panic if unauthorized by caller
// status: verified
pub fn grant_role_panics_if_caller_unauth(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    cvlr_assume!(!is_auth(caller.clone()));
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_assert!(false);
}

#[rule]
// grant role panics if caller is not admin and not admin_role
// status: verified
pub fn grant_role_panics_if_caller_not_admin_nor_admin_role(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        let caller_has_role_admin = AccessControlContract::has_role(&e, caller.clone(), role_admin_internal);
        cvlr_assume!(caller_has_role_admin.is_none());
    }
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin {
        cvlr_assume!(caller.clone()!= admin_internal);
    }
    AccessControlContract::grant_role(&e, caller.clone(), account, role.clone());
    cvlr_assert!(false);
}

#[rule]
// revoke_role panics if unauthorized by caller
// status: verified
pub fn revoke_role_panics_if_caller_unauth(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    cvlr_assume!(!is_auth(caller.clone()));
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_assert!(false);
}

#[rule]
// revoke_role panics if caller is not admin and not admin_role
// status: verified
pub fn revoke_role_panics_if_caller_not_admin_nor_admin_role(e: Env) {
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    let role = nondet_symbol();
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        let caller_has_role_admin = AccessControlContract::has_role(&e, caller.clone(), role_admin_internal);
        cvlr_assume!(caller_has_role_admin.is_none());
        clog!(caller_has_role_admin.unwrap());
    }
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin {
        cvlr_assume!(caller.clone()!= admin_internal);
        clog!(cvlr_soroban::Addr(&admin_internal));
    }
    AccessControlContract::revoke_role(&e, caller.clone(), account, role.clone());
    cvlr_assert!(false);
}


#[rule]
// revoke_role panics if account does not have the role
// status: verified
pub fn revoke_role_panics_if_account_does_not_have_role(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    let account_has_role = AccessControlContract::has_role(&e, account.clone(), role.clone());
    cvlr_assume!(account_has_role.is_none());
    AccessControlContract::revoke_role(&e, caller.clone(), account, role.clone());
    cvlr_assert!(false);
}


#[rule]
// revoke_role panics if role is empty
// status: verified
pub fn revoke_role_panics_if_role_is_empty(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    let role_member_count = AccessControlContract::get_role_member_count(&e, role.clone());
    cvlr_assume!(role_member_count == 0);
    AccessControlContract::revoke_role(&e, caller.clone(), account, role.clone());
    cvlr_assert!(false);
}

#[rule]
// renounce_role panics if unauthorized by caller
// status: verified
pub fn renounce_role_panics_if_caller_unauth(e: Env) {
    let caller = nondet_address();
    let role = nondet_symbol();
    cvlr_assume!(!is_auth(caller.clone()));
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_assert!(false);
}

#[rule]
// renounce_role panics if caller does not have the role
// status: verified
pub fn renounce_role_panics_if_caller_does_not_have_role(e: Env) {
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let role: soroban_sdk::Symbol = nondet_symbol();
    let caller_has_role: Option<u32> = AccessControlContract::has_role(&e, caller.clone(), role.clone());
    cvlr_assume!(caller_has_role.is_none());
    AccessControlContract::renounce_role(&e, caller.clone(), role.clone());
    cvlr_assert!(false);
}

#[rule]
// renounce_role panics if role is empty
// status: verified
pub fn renounce_role_panics_if_role_is_empty(e: Env) {
    let caller = nondet_address();
    let role = nondet_symbol();
    let role_member_count = AccessControlContract::get_role_member_count(&e, role.clone());
    cvlr_assume!(role_member_count == 0);
    AccessControlContract::renounce_role(&e, caller.clone(), role.clone());
    cvlr_assert!(false);
}

#[rule]
// transfer_admin_role panics if the not authorized by the admin.
// status: verified
pub fn transfer_admin_role_panics_if_unauth_by_admin(e: Env) {
    let new_admin = nondet_address();
    clog!(cvlr_soroban::Addr(&new_admin));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin.clone() {
        clog!(cvlr_soroban::Addr(&admin_internal));
        cvlr_assume!(!is_auth(admin_internal));
    }
    AccessControlContract::transfer_admin_role(&e, new_admin.clone(), live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_admin_role panics if the admin is not set.
// status: verified
pub fn transfer_admin_role_panics_if_admin_not_set(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_none());
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_admin_role panics if live_until_ledger = 0 and PendingAdmin = None
// status: verified
pub fn transfer_admin_role_panics_if_live_until_ledger_0_and_pending_admin_none(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = 0;
    let pending_admin = get_pending_admin(&e);
    cvlr_assume!(pending_admin.is_none());
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_admin_role panics if live_until_ledger = 0 and PendingAdmin != new_admin
// status: verified
pub fn transfer_admin_role_panics_if_live_until_ledger_0_and_diff_pending_admin(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = 0;
    let pending_admin = get_pending_admin(&e);
    if let Some(pending_admin_internal) = pending_admin.clone() {
        cvlr_assume!(pending_admin_internal != new_admin);
    }
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// transfer_admin_role panics if the live_until_ledger is in the past.
// status: verified
pub fn transfer_admin_role_panics_if_invalid_live_until_ledger(e: Env) {
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    cvlr_assume!(live_until_ledger < e.ledger().sequence() || live_until_ledger > e.ledger().max_live_until_ledger());
    cvlr_assume!(live_until_ledger > 0);
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_assert!(false);
}

#[rule]
// accept_admin_transfer panics if the not authorized by the pending admin.
// status: verified
pub fn accept_admin_transfer_panics_if_unauth_by_pending_admin(e: Env) {
    let pending_admin = get_pending_admin(&e);
    if let Some(pending_admin_internal) = pending_admin.clone() {
        cvlr_assume!(!is_auth(pending_admin_internal));
    }
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_assert!(false);
}

#[rule]
// accept_admin_transfer panics if the pending admin is not set.
// status: verified
pub fn accept_admin_transfer_panics_if_pending_admin_not_set(e: Env) {
    let pending_admin = get_pending_admin(&e);
    cvlr_assume!(pending_admin.is_none());
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_assert!(false);
}

#[rule]
// set_role_admin panics if not authorized by the admin
// status: verified
pub fn set_role_admin_panics_if_unauth_by_admin(e: Env) {
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(!is_auth(admin_internal));
    }
    AccessControlContract::set_role_admin(&e, role.clone(), admin_role.clone());
    cvlr_assert!(false);
}

#[rule]
// set_role_admin panics if the admin is not set
// status: verified
pub fn set_role_admin_panics_if_admin_not_set(e: Env) {
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_none());
    AccessControlContract::set_role_admin(&e, role.clone(), admin_role.clone());
    cvlr_assert!(false);
}

#[rule] 
// renounce_admin panics if not authorized by the admin.
// status: verified
pub fn renounce_admin_panics_if_unauth_by_admin(e: Env) {
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin.clone() {
        clog!(cvlr_soroban::Addr(&admin_internal));
        cvlr_assume!(!is_auth(admin_internal));
    }
    AccessControlContract::renounce_admin(&e);
    cvlr_assert!(false);
}

#[rule]
// renounce_admin panics if the admin is not set.
// status: verified
pub fn renounce_admin_panics_if_admin_not_set(e: Env) {
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_none());
    AccessControlContract::renounce_admin(&e);
    cvlr_assert!(false);
}

#[rule]
// renounce_admin panics if there is a pending adminship transfer.
// status: violated - bug!
pub fn renounce_admin_panics_if_pending_adminship_transfer(e: Env) {
    let pending_admin = get_pending_admin(&e);
    cvlr_assume!(pending_admin.is_some());
    AccessControlContract::renounce_admin(&e);
    cvlr_assert!(false);
}

// harness functions

#[rule]
// admin_function panics if not authorized by the admin.
// status: verified
pub fn admin_function_panics_if_unauth_by_admin(e: Env) {
    let admin = AccessControlContract::get_admin(&e);
    if let Some(admin_internal) = admin.clone() {
        clog!(cvlr_soroban::Addr(&admin_internal));
        cvlr_assume!(!is_auth(admin_internal));
    }
    AccessControlContract::admin_function(&e);
    cvlr_assert!(false);
}

#[rule]
// admin_function panics if admin not set
// status: verified
pub fn admin_function_panics_if_admin_not_set(e: Env) {
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_none());
    AccessControlContract::admin_function(&e);
    cvlr_assert!(false);
}

#[rule] 
// role1_func panics if caller doesn't have role
// status: violated - symbol issue
pub fn role1_func_panics_if_caller_does_not_have_role(e: Env) {
    let caller = nondet_address();
    let role1 = soroban_sdk::Symbol::new(&e, "role1");
    let caller_has_role = AccessControlContract::has_role(&e, caller.clone(), role1);
    cvlr_assume!(caller_has_role.is_none());
    AccessControlContract::role1_func(&e, caller);
    cvlr_assert!(false);
}

#[rule]
// role1_auth_func panics if caller doesn't have role
// status: violated - symbol issue
pub fn role1_auth_func_panics_if_caller_does_not_have_role(e: Env) {
    let caller = nondet_address();
    let role1 = soroban_sdk::Symbol::new(&e, "role1");
    let caller_has_role = AccessControlContract::has_role(&e, caller.clone(), role1.clone());
    cvlr_assume!(caller_has_role.is_none());
    AccessControlContract::role1_auth_func(&e, caller.clone());
    cvlr_assert!(false);
}

#[rule]
// role1_auth_func panics if caller does not authorize
// status: verified
pub fn role1_auth_func_panics_if_caller_does_not_authorize(e: Env) {
    let caller = nondet_address();
    cvlr_assume!(!is_auth(caller.clone()));
    AccessControlContract::role1_auth_func(&e, caller.clone());
    cvlr_assert!(false);
}

#[rule]
// role1_or_role2_func panics if caller doesn't have role
// status: violated - symbol issue
pub fn role1_or_role2_func_panics_if_caller_does_not_have_role(e: Env) {
    let caller = nondet_address();
    let role1 = soroban_sdk::Symbol::new(&e, "role1");
    let role2 = soroban_sdk::Symbol::new(&e, "role2");
    let caller_has_role1 = AccessControlContract::has_role(&e, caller.clone(), role1);
    let caller_has_role2 = AccessControlContract::has_role(&e, caller.clone(), role2);
    cvlr_assume!(caller_has_role1.is_none() && caller_has_role2.is_none());
    AccessControlContract::role1_or_role2_func(&e, caller.clone());
    cvlr_assert!(false);
}

#[rule]
// role1_or_role2_auth_func panics if caller doesn't have role
// status: violated - symbol issue
pub fn role1_or_role2_auth_func_panics_if_caller_does_not_have_role(e: Env) {
    let caller = nondet_address();
    let role1 = soroban_sdk::Symbol::new(&e, "role1");
    let role2 = soroban_sdk::Symbol::new(&e, "role2");
    let caller_has_role1 = AccessControlContract::has_role(&e, caller.clone(), role1);
    let caller_has_role2 = AccessControlContract::has_role(&e, caller.clone(), role2);
    cvlr_assume!(caller_has_role1.is_none() && caller_has_role2.is_none());
    AccessControlContract::role1_or_role2_auth_func(&e, caller.clone());
    cvlr_assert!(false);
}

#[rule]
// role1_or_role2_auth_func panics if caller doesn't authorize
// status: verified
pub fn role1_or_role2_auth_func_panics_if_caller_does_not_authorize(e: Env) {
    let caller = nondet_address();
    cvlr_assume!(!is_auth(caller.clone()));
    AccessControlContract::role1_or_role2_auth_func(&e, caller.clone());
    cvlr_assert!(false);
}

#[rule]
// role1_and_role2_func panics if caller1 doesn't have role
// status: violated - symbol issue
pub fn role1_and_role2_func_panics_if_caller1_does_not_have_role(e: Env) {
    let caller1 = nondet_address();
    let caller2 = nondet_address();
    let role1 = soroban_sdk::Symbol::new(&e, "role1");
    let caller1_has_role = AccessControlContract::has_role(&e, caller1.clone(), role1);
    cvlr_assume!(caller1_has_role.is_none());
    AccessControlContract::role1_and_role2_func(&e, caller1.clone(), caller2.clone());
    cvlr_assert!(false);
}

#[rule]
// role1_and_role2_func panics if caller2 doesn't have role
// status: violated - symbol issue
pub fn role1_and_role2_func_panics_if_caller2_does_not_have_role(e: Env) {
    let caller1 = nondet_address();
    let caller2 = nondet_address();
    let role2 = soroban_sdk::Symbol::new(&e, "role2");
    let caller2_has_role = AccessControlContract::has_role(&e, caller2.clone(), role2);
    cvlr_assume!(caller2_has_role.is_none());
    AccessControlContract::role1_and_role2_func(&e, caller1.clone(), caller2.clone());
    cvlr_assert!(false);
}
