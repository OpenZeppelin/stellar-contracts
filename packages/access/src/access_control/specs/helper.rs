use soroban_sdk::{Env, Address, Symbol};
use crate::access_control::{AccessControlStorageKey, storage::RoleAccountKey};
use cvlr::clog;

/// Returns `Some(Address)` if a pending owner is set, or `None` if there is no pending ownership transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_pending_admin(e: &Env) -> Option<Address> {
    let pending_admin = e.storage().temporary().get::<_, Address>(&AccessControlStorageKey::PendingAdmin);
    if let Some(pending_admin_internal) = pending_admin.clone() {
        clog!(cvlr_soroban::Addr(&pending_admin_internal));
    }
    pending_admin
}

/// Returns `Some(Address)` if an account exists at the specified index for the given role,
/// or `None` if there is no account at that index.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `role` - The role to query.
/// * `index` - The index of the account to retrieve.
pub fn get_role_account(e: &Env, role: &Symbol, index: u32) -> Option<Address> {
    let key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
        role: role.clone(),
        index,
    });
    let account = e
        .storage()
        .persistent()
        .get::<_, Address>(&key);
    if let Some(account_internal) = account.clone() {
        clog!(cvlr_soroban::Addr(&account_internal));
    }
    account
}


