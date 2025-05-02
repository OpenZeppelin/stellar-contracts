use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol};
use stellar_constants::{DAY_IN_LEDGERS, ROLE_EXTEND_AMOUNT, ROLE_TTL_THRESHOLD};

use crate::{
    emit_admin_transfer_cancelled, emit_admin_transfer_completed, emit_admin_transfer_started,
    emit_role_admin_changed, emit_role_granted, emit_role_revoked, AccessControlError,
};

// Time limit for the admin transfer in ledgers
pub const ADMIN_TRANSFER_TTL: u32 = 2 * DAY_IN_LEDGERS;
// Threshold for the admin transfer TTL extension (should be less than ADMIN_TRANSFER_TTL)
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
    RoleAdmin(Symbol),            // role -> the admin role
    Admin,
    PendingAdmin,
}

// ################## QUERY STATE ##################

/// Returns `Some(index)` if the account has the specified role,
/// where `index` is the position of the account for that role,
/// and can be used to query [`get_role_member()`].
/// Returns `None` if the account does not have the specified role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to check.
/// * `role` - The role to check for.
pub fn has_role(e: &Env, account: &Address, role: &Symbol) -> Option<u32> {
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().get(&key)
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
/// * `AccessControlError::OutOfBounds` - If the indexing is out of bounds.
pub fn get_role_member(e: &Env, role: &Symbol, index: u32) -> Address {
    let key = AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index });

    e.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::OutOfBounds))
}

/// Returns the admin role for a specific role.
/// If no admin role is explicitly set, returns `None`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to query the admin role for.
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
/// * `caller` - The address of the caller, must be the admin or has the `AdminRole` privileges for this role.
/// * `account` - The account to grant the role to.
/// * `role` - The role to grant.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller does not have enough privileges.
///
/// # Events
///
/// * topics - `["role_granted", role: Symbol, account: Address]`
/// * data - `[sender: Address]`
pub fn grant_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    ensure_if_admin_or_admin_role(e, caller, role);

    // Return early if account already has the role
    if has_role(e, account, role).is_some() {
        return;
    }

    add_to_role_enumeration(e, account, role);

    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&key, &true);
    e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);

    emit_role_granted(e, role, account, caller);
}

/// Revokes a role from an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the admin or has the `AdminRole` privileges for this role.
/// * `account` - The account to revoke the role from.
/// * `role` - The role to revoke.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `caller` does not have enough privileges.
/// * `AccessControlError::AccountNotFound` - If the `account` doesn't have the role.
/// * refer to [`remove_from_role_enumeration()`] errors.
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[sender: Address]`
pub fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    ensure_if_admin_or_admin_role(e, caller, role);

    // Check if account has the role
    if has_role(e, account, role).is_none() {
        panic_with_error!(e, AccessControlError::AccountNotFound);
    }

    remove_from_role_enumeration(e, account, role);

    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().remove(&key);

    emit_role_revoked(e, role, account, caller);
}

/// Allows an account to renounce a role assigned to itself.
/// Users can only renounce roles for their own account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the account that has the role.
/// * `role` - The role to renounce.
///
/// # Errors
///
/// * `AccessControlError::AccountNotFound` - If the `caller` doesn't have the role.
/// * refer to [`remove_from_role_enumeration()`] errors.
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[sender: Address]`
pub fn renounce_role(e: &Env, caller: &Address, role: &Symbol) {
    if has_role(e, caller, role).is_none() {
        panic_with_error!(e, AccessControlError::AccountNotFound);
    }

    remove_from_role_enumeration(e, caller, role);

    let key = AccessControlStorageKey::HasRole(caller.clone(), role.clone());
    e.storage().persistent().remove(&key);

    emit_role_revoked(e, role, caller, caller);
}

/// Initiates admin role transfer.
/// Admin privileges for the current admin are not revoked until the
/// recipient accepts the transfer.
/// Overrides the previous pending transfer if there is one.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the admin.
/// * `new_admin` - The account to transfer the admin privileges to.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `caller` is not the admin.
///
/// # Events
///
/// * topics - `["admin_transfer_started", current_admin: Address]`
/// * data - `[new_admin: Address]`
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

    emit_admin_transfer_started(e, caller, new_admin);
}

/// Cancels a pending admin role transfer if it is not accepted yet.
/// This can only be called by the current admin.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the admin.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `caller` is not the admin.
///
/// # Events
///
/// * topics - `["admin_transfer_cancelled", admin: Address]`
/// * data - `[]` (empty data)
pub fn cancel_transfer_admin_role(e: &Env, caller: &Address) {
    if caller != &get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    e.storage().temporary().remove(&AccessControlStorageKey::PendingAdmin);

    emit_admin_transfer_cancelled(e, caller);
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
/// * `AccessControlError::Unauthorized` - If the `caller` is not the pending admin.
///
/// # Events
///
/// * topics - `["admin_transfer_completed", new_admin: Address]`
/// * data - `[previous_admin: Address]`
pub fn accept_admin_transfer(e: &Env, caller: &Address) {
    let pending_admin = e
        .storage()
        .temporary()
        .get::<_, Address>(&AccessControlStorageKey::PendingAdmin)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::NoPendingAdminTransfer));

    if &pending_admin != caller {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    let previous_admin = get_admin(e);

    e.storage().temporary().remove(&AccessControlStorageKey::PendingAdmin);
    e.storage().instance().set(&AccessControlStorageKey::Admin, caller);

    emit_admin_transfer_completed(e, &previous_admin, caller);
}

/// Sets `admin_role` as the admin role for `role`.
/// The admin role for a role controls who can grant and revoke that role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the admin.
/// * `role` - The role to set the admin for.
/// * `admin_role` - The role that will be the admin.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `caller` is not the admin.
///
/// # Events
///
/// * topics - `["role_admin_changed", role: Symbol]`
/// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
pub fn set_role_admin(e: &Env, caller: &Address, role: &Symbol, admin_role: &Symbol) {
    if caller != &get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    let key = AccessControlStorageKey::RoleAdmin(role.clone());

    // Get previous admin role if exists
    let previous_admin_role =
        e.storage().persistent().get::<_, Symbol>(&key).unwrap_or_else(|| Symbol::new(e, ""));

    e.storage().persistent().set(&key, admin_role);
    e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);

    emit_role_admin_changed(e, role, &previous_admin_role, admin_role);
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
/// * `AccessControlError::AccountNotFound` - If the role has no members or the `account` doesn't have the role.
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

// TODO: inline docs
pub fn ensure_if_admin_or_admin_role(e: &Env, caller: &Address, role: &Symbol) {
    let is_admin = caller == &get_admin(e);
    let is_admin_role = match get_role_admin(e, role) {
        Some(admin_role) => has_role(e, caller, &admin_role).is_some(),
        None => false,
    };

    // If caller is not the contract admin and does not have the admin role for this role
    if !is_admin && !is_admin_role {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }
}
