use soroban_sdk::{panic_with_error, Address, Env, Symbol};
use stellar_constants::{DAY_IN_LEDGERS, ROLE_EXTEND_AMOUNT, ROLE_TTL_THRESHOLD};

use crate::{AccessControlError, AccessControlStorageKey, RoleAccountKey};

// Time limit for admin transfer in ledgers
pub const ADMIN_TRANSFER_TTL: u32 = 2 * DAY_IN_LEDGERS;
// Threshold for admin transfer TTL extension (should be less than ADMIN_TRANSFER_TTL)
pub const ADMIN_TRANSFER_THRESHOLD: u32 = DAY_IN_LEDGERS;

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
    RoleAccountsCount(Symbol),    // role -> count
    RoleAdmin(Symbol),            // role -> admin role
    Admin,
    PendingAdmin,
}

// ################## QUERY STATE ##################

/// Returns true if the account has the specified role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to check.
/// * `role` - The role to check for.
pub fn has_role(e: &Env, account: &Address, role: &Symbol) -> bool {
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().has(&key)
}

/// Returns the admin account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * `AccessControlError::AccountNotFound` - If no admin account is set.
pub fn get_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&AccessControlStorageKey::Admin)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::AccountNotFound))
}

/// Returns the total number of accounts that have the specified role.
/// If the role does not exist, it returns 0.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to get the count for.
pub fn get_role_member_count(e: &Env, role: &Symbol) -> u32 {
    let count_key = AccessControlStorageKey::RoleAccountsCount(role.clone());
    e.storage().persistent().get(&count_key).unwrap_or(0)
}

/// Returns the account at the specified index for a given role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to query.
/// * `index` - The index of the account to retrieve.
///
/// # Errors
///
/// * `AccessControlError::AccountNotFound` - If the index is out of bounds or the account doesn't exist.
pub fn get_role_member(e: &Env, role: &Symbol, index: u32) -> Address {
    let count = get_role_member_count(e, role);

    if index >= count {
        panic_with_error!(e, AccessControlError::AccountNotFound);
    }

    let key = AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index });

    e.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::AccountNotFound))
}

/// Returns the admin role for a specific role.
/// If no admin role is explicitly set, returns `None`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to query the admin for.
pub fn get_role_admin(e: &Env, role: &Symbol) -> Option<Symbol> {
    let key = AccessControlStorageKey::RoleAdmin(role.clone());
    e.storage().persistent().get(&key)
}

// ################## CHANGE STATE ##################

/// Grants a role to an account.
/// Creates the role if it does not exist.
/// Returns early if the account already has the role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be admin.
/// * `account` - The account to grant the role to.
/// * `role` - The role to grant.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller is not admin.
pub fn grant_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    let admin_role = get_role_admin(e, role);

    // If caller is not the contract admin and does not have the admin role for this role
    if caller != &get_admin(e) && !has_role(e, caller, &admin_role) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Return early if account already has the role
    if has_role(e, account, role) {
        return;
    }

    add_to_role_enumeration(e, account, role);

    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&key, &true);
    e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
}

/// Revokes a role from an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be admin.
/// * `account` - The account to revoke the role from.
/// * `role` - The role to revoke.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller is not admin.
/// * `AccessControlError::AccountNotFound` - If the account doesn't have the role.
pub fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    let admin_role = get_role_admin(e, role);

    // If caller is not the contract admin and does not have the admin role for this role
    if caller != &get_admin(e) && !has_role(e, caller, &admin_role) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Check if account has the role
    if !has_role(e, account, role) {
        panic_with_error!(e, AccessControlError::AccountNotFound);
    }

    remove_from_role_enumeration(e, account, role);

    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().remove(&key);
}

/// Initiates admin role transfer.
/// Admin privileges for the current admin are not revoked until the
/// recipient accepts the transfer.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be admin.
/// * `new_admin` - The account to transfer admin privileges to.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller is not admin.
pub fn transfer_admin_role(e: &Env, caller: &Address, new_admin: &Address) {
    if caller != &get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    // Store the new admin address in temporary storage
    e.storage().temporary().set(&AccessControlStorageKey::PendingAdmin, new_admin);
    e.storage().temporary().extend_ttl(
        &AccessControlStorageKey::PendingAdmin,
        ADMIN_TRANSFER_THRESHOLD,
        ADMIN_TRANSFER_TTL,
    );
}

/// Cancels a pending admin role transfer if it is not accepted yet.
/// This can only be called by the current admin.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be admin.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller is not admin.
pub fn cancel_transfer_admin_role(e: &Env, caller: &Address) {
    if caller != &get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    e.storage().temporary().remove(&AccessControlStorageKey::PendingAdmin);
}

/// Completes the 2-step admin transfer.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the pending admin.
///
/// # Errors
///
/// * `AccessControlError::NoPendingAdminTransfer` - If no pending admin transfer is set.
/// * `AccessControlError::Unauthorized` - If the caller is not the pending admin.
pub fn accept_admin_transfer(e: &Env, caller: &Address) {
    let pending_admin = e
        .storage()
        .temporary()
        .get(&AccessControlStorageKey::PendingAdmin)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::NoPendingAdminTransfer));

    if &pending_admin != caller {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    e.storage().temporary().remove(&AccessControlStorageKey::PendingAdmin);

    e.storage().instance().set(&AccessControlStorageKey::Admin, caller);
}

/// Sets `admin_role` as the admin role for `role`.
/// The admin role for a role controls who can grant and revoke that role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be admin.
/// * `role` - The role to set the admin for.
/// * `admin_role` - The role that will be the admin.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller is not admin.
pub fn set_role_admin(e: &Env, caller: &Address, role: &Symbol, admin_role: &Symbol) {
    if caller != &get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    let key = AccessControlStorageKey::RoleAdmin(role.clone());
    e.storage().persistent().set(&key, admin_role);
    e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
}

// ################## LOW-LEVEL HELPERS ##################

/// Adds an account to role enumeration.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to add to the role.
/// * `role` - The role to add the account to.
pub fn add_to_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
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

/// Removes an account from role enumeration.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to remove from the role.
/// * `role` - The role to remove the account from.
///
/// # Errors
///
/// * `AccessControlError::AccountNotFound` - If the role has no members or the account doesn't have the role.
pub fn remove_from_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
    // Get the current count of accounts with this role
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
    let last_index = count - 1;
    let last_key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
        role: role.clone(),
        index: last_index,
    });

    // Swap the to be removed account with the last account, then delete the last account
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
