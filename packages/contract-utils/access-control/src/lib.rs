//! Access control module for Soroban contracts
//!
//! This module provides functionality to manage role-based access control in Soroban contracts.
//!
//! # Usage
//!
//! One can create new methods for their contract and specify which roles can do what.
//! For example, one could create a role `mint_admins` and specify a 2nd level administration.
//! This group may have access to `revoke_mint_role` and `grant_mint_role` methods.
//!
//! In order to do that:
//! 1. the admin will create the `minter_admin` role and specify the accounts for that with [`grant_role()`] function.
//!
//! Then, the new methods can be implemented for the contract may look like this:
//!
//! ```rust
//! #[has_role(caller, "minter_admin")]
//! pub fn grant_mint_role(e: &Env, caller: Address) {
//!     ...
//! }
//! ```
//!
//! If multi-admin setup is wanted, it can be achieved in a similar way by creating a new admin role
//! and assigning accounts to it.

use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol};
use stellar_constants::{ROLE_EXTEND_AMOUNT, ROLE_TTL_THRESHOLD};

#[contracttype]
pub struct RoleAccountKey {
    pub role: Symbol,
    pub index: u32,
}

/// Storage keys for the data associated with the access control
#[contracttype]
pub enum AccessControlStorageKey {
    RoleAccounts(RoleAccountKey), // (role, index) -> Address
    HasRole(Address, Symbol),     // (account, role) -> index
    RoleAccountsCount(Symbol),    // (role) -> u32
    Admin,
    PendingAdmin, // Temporary storage for pending admin transfer
}

/// Errors that can be returned by the access control module
#[contracttype]
pub enum AccessControlError {
    Unauthorized,
    RoleNotFound,
    AccountNotFound,
}

// ################## QUERY STATE ##################

/// Returns true if the account has the specified role.
///
/// Admin role does not need to be created, it exists by default, and not
/// related to a unique symbol. Use `get_admin` to query admin role.
pub fn has_role(e: &Env, account: &Address, role: &Symbol) -> bool {
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().has(&key)
}

/// Returns the admin account
pub fn get_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&AccessControlStorageKey::Admin)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::AccountNotFound))
}

// ################## CHANGE STATE ##################

/// Grants a role to an account
/// Creates the role if it does not exist
/// Returns early if the account already has the role
/// Caller must be admin
pub fn grant_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    // Check if caller is admin
    if caller != &AccessControl::get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Return early if account already has the role
    // TODO: question: should we return an error instead?
    if AccessControl::has_role(e, account, role) {
        return;
    }

    // Add account to role enumeration
    AccessControl::add_to_role_enumeration(e, account, role);

    // Set the role for the account
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&key, &true);
    e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
}

/// Revokes a role from an account
/// Errors if the account does not have the role
/// Caller must be admin
pub fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    // Check if caller is admin
    if caller != &AccessControl::get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Check if account has the role
    if !AccessControl::has_role(e, account, role) {
        panic_with_error!(e, AccessControlError::AccountNotFound);
    }

    // Remove account from role enumeration
    AccessControl::remove_from_role_enumeration(e, account, role);

    // Remove the role from the account
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().remove(&key);
}

/// Caller must be admin
/// Admin privileges for the current admin are not revoked until the
/// recipient accepts the transfer
pub fn transfer_admin_role(e: &Env, caller: &Address, new_admin: &Address) {
    // Check if caller is admin
    if caller != &AccessControl::get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Store the new admin address in temporary storage
    e.storage().temporary().set(&AccessControlStorageKey::PendingAdmin, new_admin);
    e.storage().temporary().extend_ttl(key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
}

/// Completes the 2-step admin transfer
pub fn accept_admin_transfer(e: &Env, caller: &Address) {
    // Check if caller is the pending admin
    let pending_admin = e
        .storage()
        .temporary()
        .get(&AccessControlStorageKey::PendingAdmin)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::AccountNotFound));

    if &pending_admin != caller {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Remove the pending admin from temporary storage
    e.storage().temporary().remove(&AccessControlStorageKey::PendingAdmin);

    // Update the admin in instance storage
    e.storage().instance().set(&AccessControlStorageKey::Admin, caller);
}

// ################## LOW-LEVEL HELPERS ##################

/// Adds an account to role enumeration
fn add_to_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
    // Get the current count of accounts with this role
    let count_key = AccessControlStorageKey::RoleAccountsCount(role.clone());
    let count = e.storage().persistent().get(&count_key).unwrap_or(0);

    // Add the account to the enumeration
    let new_key =
        AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index: count });
    e.storage().persistent().set(&new_key, account);
    e.storage().persistent().extend_ttl(&new_key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);

    // Store the index for the account in HasRole
    let has_role_key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&has_role_key, &count);
    e.storage().persistent().extend_ttl(&has_role_key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);

    // Update the count
    e.storage().persistent().set(&count_key, &(count + 1));
    e.storage().persistent().extend_ttl(&count_key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
}

/// Removes an account from role enumeration
fn remove_from_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
    let count_key = AccessControlStorageKey::RoleAccountsCount(role.clone());
    let count = e.storage().persistent().get(&count_key).unwrap_or(0);
    if count == 0 {
        panic_with_error!(e, AccessControlError::AccountNotFound);
    }

    // Get the index of the account to remove
    let to_be_removed_has_role_key =
        AccessControlStorageKey::HasRole(account.clone(), role.clone());
    let to_be_removed_index =
        e.storage().persistent().get::<_, u32>(&to_be_removed_has_role_key).unwrap();

    // Get the index of the last account for that role
    let last_key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
        role: role.clone(),
        index: last_index,
    });

    // Swap the to be removed account with the last account, then delete the last account
    let last_index = count - 1;
    if to_be_removed_index != last_index {
        let last_account = e.storage().persistent().get::<_, Address>(&last_key).unwrap();

        // Swap
        let to_be_removed_key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
            role: role.clone(),
            index: to_be_removed_index,
        });
        e.storage().persistent().set(&to_be_removed_key, &last_account);

        // Update HasRole for the swapped account
        let last_account_has_role_key =
            AccessControlStorageKey::HasRole(last_account.clone(), role.clone());
        e.storage().persistent().set(&last_account_has_role_key, &to_be_removed_index);
    }

    // Remove the last account
    e.storage().persistent().remove(&last_key);
    e.storage().persistent().remove(&to_be_removed_has_role_key);

    // Update the count
    e.storage().persistent().set(&count_key, &last_index);
    e.storage().persistent().extend_ttl(&count_key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
}
