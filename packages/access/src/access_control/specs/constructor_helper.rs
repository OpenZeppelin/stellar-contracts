use cvlr::cvlr_assume;
use soroban_sdk::{Env, Address, Symbol};

use crate::access_control::{AccessControl, specs::access_control_contract::AccessControlContract};

use crate::access_control::storage::{AccessControlStorageKey, RoleAccountKey};

pub fn before_constructor_no_admin(e: &Env) {
    let key = AccessControlStorageKey::Admin;
    let admin = e.storage().persistent().get::<_, Address>(&key);
    cvlr_assume!(admin.is_none());
}

pub fn before_constructor_no_pending_admin(e: &Env) {
    let key = AccessControlStorageKey::PendingAdmin;
    let pending_admin = e.storage().temporary().get::<_, Address>(&key);
    cvlr_assume!(pending_admin.is_none());
}

pub fn before_constructor_no_role_count(e: &Env, role: &Symbol) {
    let key = AccessControlStorageKey::RoleAccountsCount(role.clone());
    let count = e.storage().persistent().get::<_, u32>(&key);
    cvlr_assume!(count.is_none());
}

pub fn before_constructor_no_role_admin(e: &Env, role: &Symbol) {
    let key = AccessControlStorageKey::RoleAdmin(role.clone());
    let admin = e.storage().persistent().get::<_, Symbol>(&key);
    cvlr_assume!(admin.is_none());
}

pub fn before_constructor_no_has_role(e: &Env, account: Address, role: Symbol) {
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    let has_role = e.storage().persistent().get::<_, u32>(&key);
    cvlr_assume!(has_role.is_none());
}

pub fn before_constructor_no_role_accounts(e: &Env, role: Symbol, index: u32) {
    let key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
        role,
        index,
    });
    let account = e.storage().persistent().get::<_, Address>(&key);
    cvlr_assume!(account.is_none());
}

