use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol};
use stellar_constants::{ROLE_EXTEND_AMOUNT, ROLE_TTL_THRESHOLD};

use crate::{
    emit_admin_transfer_completed, emit_admin_transfer_initiated, emit_role_admin_changed,
    emit_role_granted, emit_role_revoked, AccessControlError,
};

/// Storage key for enumeration of accounts per role.
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
/// * `caller` - The address of the caller, must be the admin or has the
///   `AdminRole` privileges for this role.
/// * `account` - The account to grant the role to.
/// * `role` - The role to grant.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller does not have enough
///   privileges.
///
/// # Events
///
/// * topics - `["role_granted", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
///
/// # Notes
///
/// * Authorization for `caller` is required.
pub fn grant_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    caller.require_auth();
    ensure_if_admin_or_admin_role(e, caller, role);

    // Return early if account already has the role
    if has_role(e, account, role).is_some() {
        return;
    }

    let index = add_to_role_enumeration(e, account, role);

    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&key, &index);
    e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);

    emit_role_granted(e, role, account, caller);
}

/// Revokes a role from an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller, must be the admin or has the
///   `AdminRole` privileges for this role.
/// * `account` - The account to revoke the role from.
/// * `role` - The role to revoke.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `caller` does not have enough
///   privileges.
/// * `AccessControlError::AccountNotFound` - If the `account` doesn't have the
///   role.
/// * refer to [`remove_from_role_enumeration()`] errors.
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
///
/// # Notes
///
/// * Authorization for `caller` is required.
pub fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    caller.require_auth();
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
/// * `caller` - The address of the caller, must be the account that has the
///   role.
/// * `role` - The role to renounce.
///
/// # Errors
///
/// * `AccessControlError::AccountNotFound` - If the `caller` doesn't have the
///   role.
/// * refer to [`remove_from_role_enumeration()`] errors.
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
///
/// # Notes
///
/// * Authorization for `caller` is required.
pub fn renounce_role(e: &Env, caller: &Address, role: &Symbol) {
    caller.require_auth();
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
/// * `admin` - The address of the admin.
/// * `new_admin` - The account to transfer the admin privileges to.
/// * `live_until_ledger` - The ledger number at which the pending transfer
///   expires. If `live_until_ledger` is `0`, the pending transfer is cancelled.
///   `live_until_ledger` argument is implicitly bounded by the maximum allowed
///   TTL extension for a temporary storage entry and specifying a higher value
///   will cause the code to panic.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `admin` is not actually the
///   admin.
/// * `AccessControlError::NoPendingAdminTransfer` - If tried to cancel the
///   pending admin transfer when there is no pending admin transfer.
///
/// # Events
///
/// * topics - `["admin_transfer_initiated", current_admin: Address]`
/// * data - `[new_admin: Address, live_until_ledger: u32]`
///
/// # Notes
///
/// * Authorization for `admin` is required.
pub fn transfer_admin_role(e: &Env, admin: &Address, new_admin: &Address, live_until_ledger: u32) {
    admin.require_auth();
    if *admin != get_admin(e) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }

    let key = AccessControlStorageKey::PendingAdmin;

    if live_until_ledger == 0 {
        let Some(pending_new_admin) = e.storage().temporary().get::<_, Address>(&key) else {
            panic_with_error!(e, AccessControlError::NoPendingAdminTransfer)
        };
        e.storage().temporary().remove(&key);

        emit_admin_transfer_initiated(e, admin, &pending_new_admin, live_until_ledger);
        return;
    }

    let current_ledger = e.ledger().sequence();

    if live_until_ledger < current_ledger {
        panic_with_error!(e, AccessControlError::InvalidLiveUntilLedger);
    }

    let live_for = live_until_ledger - current_ledger;

    // Store the new admin address in temporary storage
    e.storage().temporary().set(&key, new_admin);
    e.storage().temporary().extend_ttl(&key, live_for, live_for);

    emit_admin_transfer_initiated(e, admin, new_admin, live_until_ledger);
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
/// * `AccessControlError::NoPendingAdminTransfer` - If no pending admin
///   transfer is set.
/// * `AccessControlError::Unauthorized` - If the `caller` is not the pending
///   admin.
///
/// # Events
///
/// * topics - `["admin_transfer_completed", new_admin: Address]`
/// * data - `[previous_admin: Address]`
///
/// # Notes
///
/// * Authorization for `caller` is required.
pub fn accept_admin_transfer(e: &Env, caller: &Address) {
    caller.require_auth();
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
/// * `admin` - The address of the admin.
/// * `role` - The role to set the admin for.
/// * `admin_role` - The role that will be the admin.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the `admin` is not actually the
///   admin.
///
/// # Events
///
/// * topics - `["role_admin_changed", role: Symbol]`
/// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
///
/// # Notes
///
/// * Authorization for `admin` is required.
pub fn set_role_admin(e: &Env, admin: &Address, role: &Symbol, admin_role: &Symbol) {
    admin.require_auth();
    if admin != &get_admin(e) {
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

/// Adds an account to role enumeration. Returns the previous count.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to add to the role.
/// * `role` - The role to add the account to.
pub fn add_to_role_enumeration(e: &Env, account: &Address, role: &Symbol) -> u32 {
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

    count
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
/// * `AccessControlError::AccountNotFound` - If the role has no members or the
///   `account` doesn't have the role.
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
    let to_be_removed_index = e
        .storage()
        .persistent()
        .get::<_, u32>(&to_be_removed_has_role_key)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::AccountNotFound));

    // Get the index of the last account for that role
    let last_index = count - 1;
    let last_key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
        role: role.clone(),
        index: last_index,
    });

    // Swap the to be removed account with the last account, then delete the last
    // account
    if to_be_removed_index != last_index {
        let last_account = e
            .storage()
            .persistent()
            .get::<_, Address>(&last_key)
            .expect("we ensured count to be 1 at this point");

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

/// Ensures that the caller is either the contract admin or has the admin role
/// for the specified role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller to check permissions for.
/// * `role` - The role to check admin privileges for.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller is neither the contract
///   admin nor has the admin role.
pub fn ensure_if_admin_or_admin_role(e: &Env, caller: &Address, role: &Symbol) {
    let is_admin = caller == &get_admin(e);
    let is_admin_role = match get_role_admin(e, role) {
        Some(admin_role) => has_role(e, caller, &admin_role).is_some(),
        None => false,
    };

    if !is_admin && !is_admin_role {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }
}

/// Ensures that the caller has the specified role.
/// This function is used to check if an account has a specific role.
/// The main purpose of this function is to act as a helper for the
/// `#[has_role]` macro.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller to check the role for.
/// * `role` - The role to check for.
///
/// # Errors
///
/// * `AccessControlError::Unauthorized` - If the caller does not have the
///   specified role.
pub fn ensure_role(e: &Env, caller: &Address, role: &Symbol) {
    if has_role(e, caller, role).is_none() {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }
}
