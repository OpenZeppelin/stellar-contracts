
use cvlr::{cvlr_assert, cvlr_assume,cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_symbol, is_auth};
use cvlr::nondet::{Nondet, nondet};
use cvlr_soroban_derive::rule;
use cvlr::clog;

use crate::access_control::storage::{AccessControlStorageKey, RoleAccountKey};
use soroban_sdk::{Env, Address, Symbol};
use crate::access_control::{AccessControl, specs::{access_control_contract::AccessControlContract, helper::get_pending_admin}};

// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to consider also panicking paths.

// storage setup

// im a bit unsure about storage setup in cases where there are options,
// is the case of None ignored the way we do this?

pub fn storage_setup_admin(e: Env) {
    let admin = nondet_address();
    e.storage().instance().set(&AccessControlStorageKey::Admin, &admin);
}

pub fn storage_setup_pending_admin(e: Env) {
    let pending_admin = nondet_address();
    e.storage().temporary().set(&AccessControlStorageKey::PendingAdmin, &pending_admin);
}

pub fn storage_setup_pending_admin_none(e: Env) {
    let pending_admin: Option<Address> = None::<Address>;
    e.storage().temporary().set(&AccessControlStorageKey::PendingAdmin, &pending_admin.clone());
}

pub fn storage_setup_role_admin(e: Env, role: Symbol) {
    let role_admin_key: AccessControlStorageKey = AccessControlStorageKey::RoleAdmin(role.clone());
    let symbol = nondet_symbol();
    e.storage().persistent().set(&role_admin_key, &symbol);   
}

pub fn storage_setup_role_counts(e: Env, role: Symbol) {
    let role_accounts_count_key: AccessControlStorageKey = AccessControlStorageKey::RoleAccountsCount(role.clone());
    let nondet_count : u32 = nondet();
    e.storage().persistent().set(&role_accounts_count_key, &nondet_count); 
}

pub fn storage_setup_account_has_role(e: Env, account: Address, role: Symbol) {
    let has_role_key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    let nondet_index_account : u32 = nondet();
    e.storage().persistent().set(&has_role_key, &nondet_index_account); 
}

pub fn storage_setup_caller_has_role_admin(e: Env, caller: Address, role: Symbol) {
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        let caller_has_role_admin_key = AccessControlStorageKey::HasRole(caller.clone(), role_admin_internal.clone());
        let nondet_index : u32 = nondet();
        e.storage().persistent().set(&caller_has_role_admin_key, &nondet_index);    
    }
}

pub fn storage_setup_last_account(e: Env, role: Symbol) {
    let count = AccessControlContract::get_role_member_count(&e, role.clone());
    let last_index = count - 1;
    let last_key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
        role: role.clone(),
        index: last_index,
    });
    let last_account = nondet_address();
    e.storage().persistent().set(&last_key, &last_account);
}

// package functions

#[rule]
// requires
// storage setup
// caller auth
// caller is admin or has admin role
// status: verified (13 minutes)
pub fn grant_role_non_panic(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();

    storage_setup_admin(e.clone());
    storage_setup_role_admin(e.clone(), role.clone());
    storage_setup_account_has_role(e.clone(), account.clone(), role.clone());
    storage_setup_caller_has_role_admin(e.clone(), caller.clone(), role.clone());

    cvlr_assume!(is_auth(caller.clone()));
    let admin = AccessControlContract::get_admin(&e);
    let mut caller_equals_admin = false;
    if let Some(admin_internal) = admin {
        caller_equals_admin = caller.clone() == admin_internal;
    }
    let mut caller_has_role_admin = false;
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        caller_has_role_admin = AccessControlContract::has_role(&e, caller.clone(), role_admin_internal).is_some();
    }
    cvlr_assume!(caller_equals_admin || caller_has_role_admin);
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn grant_role_non_panic_sanity(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();

    storage_setup_admin(e.clone());
    storage_setup_role_admin(e.clone(), role.clone());
    storage_setup_role_counts(e.clone(), role.clone());
    storage_setup_account_has_role(e.clone(), account.clone(), role.clone());
    storage_setup_caller_has_role_admin(e.clone(), caller.clone(), role.clone());

    cvlr_assume!(is_auth(caller.clone()));
    let admin = AccessControlContract::get_admin(&e);
    let mut caller_equals_admin = false;
    if let Some(admin_internal) = admin {
        caller_equals_admin = caller.clone() == admin_internal;
    }
    let mut caller_has_role_admin = false;
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        caller_has_role_admin = AccessControlContract::has_role(&e, caller.clone(), role_admin_internal).is_some();
    }
    cvlr_assume!(caller_equals_admin || caller_has_role_admin);
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// storage setup
// auth by caller
// caller is admin or has admin_role
// account has the role
// role is not empty
// status: verified 
// when using -split false
pub fn revoke_role_non_panic(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();

    storage_setup_admin(e.clone());
    storage_setup_role_admin(e.clone(), role.clone());
    storage_setup_role_counts(e.clone(), role.clone());
    storage_setup_account_has_role(e.clone(), account.clone(), role.clone());
    storage_setup_caller_has_role_admin(e.clone(), caller.clone(), role.clone());
    storage_setup_last_account(e.clone(), role.clone());
    
    cvlr_assume!(is_auth(caller.clone()));
    let admin = AccessControlContract::get_admin(&e);
    let mut caller_equals_admin = false;
    if let Some(admin_internal) = admin {
        caller_equals_admin = caller.clone() == admin_internal;
    }
    let mut caller_has_role_admin = false;
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        caller_has_role_admin = AccessControlContract::has_role(&e, caller.clone(), role_admin_internal).is_some();
    }
    cvlr_assume!(caller_equals_admin || caller_has_role_admin);
    let account_has_role = AccessControlContract::has_role(&e, account.clone(), role.clone());
    cvlr_assume!(account_has_role.is_some());
    let role_member_count = AccessControlContract::get_role_member_count(&e, role.clone());
    cvlr_assume!(role_member_count > 0);
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn revoke_role_non_panic_sanity(e: Env) {
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();

    storage_setup_admin(e.clone());
    storage_setup_role_admin(e.clone(), role.clone());
    storage_setup_role_counts(e.clone(), role.clone());
    storage_setup_account_has_role(e.clone(), account.clone(), role.clone());
    storage_setup_caller_has_role_admin(e.clone(), caller.clone(), role.clone());
    storage_setup_last_account(e.clone(), role.clone());

    cvlr_assume!(is_auth(caller.clone()));
    let admin = AccessControlContract::get_admin(&e);
    let mut caller_equals_admin = false;
    if let Some(admin_internal) = admin {
        caller_equals_admin = caller.clone() == admin_internal;
    }
    let mut caller_has_role_admin = false;
    let role_admin = AccessControlContract::get_role_admin(&e, role.clone());
    if let Some(role_admin_internal) = role_admin {
        caller_has_role_admin = AccessControlContract::has_role(&e, caller.clone(), role_admin_internal).is_some();
    }
    cvlr_assume!(caller_equals_admin || caller_has_role_admin);
    let account_has_role = AccessControlContract::has_role(&e, account.clone(), role.clone());
    cvlr_assume!(account_has_role.is_some());
    let role_member_count = AccessControlContract::get_role_member_count(&e, role.clone());
    cvlr_assume!(role_member_count > 0);
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// storage setup
// auth by caller
// caller has the role
// role is not empty
// status: verified
pub fn renounce_role_non_panic(e: Env) { 
    let caller = nondet_address();
    let role = nondet_symbol();

    storage_setup_role_counts(e.clone(), role.clone());
    storage_setup_account_has_role(e.clone(), caller.clone(), role.clone());
    storage_setup_last_account(e.clone(), role.clone());

    cvlr_assume!(is_auth(caller.clone()));
    let caller_has_role = AccessControlContract::has_role(&e, caller.clone(), role.clone());
    cvlr_assume!(caller_has_role.is_some());
    let role_member_count = AccessControlContract::get_role_member_count(&e, role.clone());
    cvlr_assume!(role_member_count > 0);
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn renounce_role_non_panic_sanity(e: Env) { 
    let caller = nondet_address();
    let role = nondet_symbol();

    storage_setup_role_counts(e.clone(), role.clone());
    storage_setup_account_has_role(e.clone(), caller.clone(), role.clone());
    storage_setup_last_account(e.clone(), role.clone());
    
    cvlr_assume!(is_auth(caller.clone()));
    let caller_has_role = AccessControlContract::has_role(&e, caller.clone(), role.clone());
    cvlr_assume!(caller_has_role.is_some());
    let role_member_count = AccessControlContract::get_role_member_count(&e, role.clone());
    cvlr_assume!(role_member_count > 0);
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// storage setup
// admin exists
// admin auth
// if there is a pending owner they are the same
// live until ledger is appropriate
// status: verified
pub fn transfer_admin_role_non_panic(e: Env) {
    let new_admin = nondet_address().clone();
    let live_until_ledger = u32::nondet();

    storage_setup_pending_admin(e.clone());
    storage_setup_admin(e.clone());

    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_some());
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(is_auth(admin_internal));
    }

    let pending_admin = get_pending_admin(&e);
    if let Some(pending_admin_internal) = pending_admin.clone() {
        cvlr_assume!(pending_admin_internal == new_admin);
    }

    if live_until_ledger == 0 {
        cvlr_assume!(pending_admin.is_some());
    }
    else {
        cvlr_assume!(live_until_ledger >= e.ledger().sequence());
        cvlr_assume!(live_until_ledger <= e.ledger().max_live_until_ledger());
    }

    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn transfer_admin_role_non_panic_sanity(e: Env) {
    let new_admin = nondet_address().clone();
    let live_until_ledger = u32::nondet();

    storage_setup_pending_admin(e.clone());
    storage_setup_admin(e.clone());

    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_some());
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(is_auth(admin_internal));
    }

    let pending_admin = get_pending_admin(&e);
    if let Some(pending_admin_internal) = pending_admin.clone() {
        cvlr_assume!(pending_admin_internal == new_admin);
    }

    if live_until_ledger == 0 {
        cvlr_assume!(pending_admin.is_some());
    }
    else {
        cvlr_assume!(live_until_ledger >= e.ledger().sequence());
        cvlr_assume!(live_until_ledger <= e.ledger().max_live_until_ledger());
    }

    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// storage setup
// pending admin exists
// pending admin auth
// status: verified
pub fn accept_admin_transfer_non_panic(e: Env) {

    storage_setup_pending_admin(e.clone());
    storage_setup_admin(e.clone());
        
    let pending_admin = get_pending_admin(&e);
    cvlr_assume!(pending_admin.is_some());
    if let Some(pending_admin_internal) = pending_admin.clone() {
        cvlr_assume!(is_auth(pending_admin_internal));
    }
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn accept_admin_transfer_non_panic_sanity(e: Env) {
    storage_setup_pending_admin(e.clone());
    storage_setup_admin(e.clone());
        
    let pending_admin = get_pending_admin(&e);
    cvlr_assume!(pending_admin.is_some());
    if let Some(pending_admin_internal) = pending_admin.clone() {
        cvlr_assume!(is_auth(pending_admin_internal));
    }
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// storage setup
// admin exists
// admin auth
// status: verified
pub fn set_role_admin_non_panic(e: Env) {
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    storage_setup_admin(e.clone());
    storage_setup_role_admin(e.clone(), role.clone());
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_some());
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(is_auth(admin_internal));
    }
    AccessControlContract::set_role_admin(&e, role.clone(), admin_role.clone());
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn set_role_admin_non_panic_sanity(e: Env) {
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    storage_setup_admin(e.clone());
    storage_setup_role_admin(e.clone(), role.clone());
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_some());
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(is_auth(admin_internal));
    }
    AccessControlContract::set_role_admin(&e, role.clone(), admin_role.clone());
    cvlr_satisfy!(true);
}

#[rule]
// requires
// storage setup
// admin exists
// admin auth
// no pending admin
// status: verified
pub fn renounce_admin_non_panic(e: Env) {
    storage_setup_admin(e.clone());
    storage_setup_pending_admin_none(e.clone());
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_some());
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(is_auth(admin_internal));
    }
    AccessControlContract::renounce_admin(&e);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: verified
pub fn renounce_admin_non_panic_sanity(e: Env) {
    storage_setup_admin(e.clone());
    storage_setup_pending_admin_none(e.clone());
    let admin = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin.is_some());
    if let Some(admin_internal) = admin.clone() {
        cvlr_assume!(is_auth(admin_internal));
    }
    AccessControlContract::renounce_admin(&e);
    cvlr_satisfy!(true);
}