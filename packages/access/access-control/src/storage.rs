use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol};
use stellar_constants::{ROLE_EXTEND_AMOUNT, ROLE_TTL_THRESHOLD};
use stellar_role_transfer::{accept_transfer, transfer_role};

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
/// and can be used to query [`get_role_member`].
/// Returns `None` if the account does not have the specified role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to check.
/// * `role` - The role to check for.
pub fn has_role(e: &Env, account: &Address, role: &Symbol) -> Option<u32> {
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());

    // extend ttl if `Some(index)`
    e.storage().persistent().get(&key).inspect(|_| {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT)
    })
}

/// Returns the admin account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * [`AccessControlError::AdminNotSet`] - If no admin account is set.
pub fn get_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&AccessControlStorageKey::Admin)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::AdminNotSet))
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
    if let Some(count) = e.storage().persistent().get(&count_key) {
        e.storage().persistent().extend_ttl(&count_key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
        count
    } else {
        0
    }
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
/// * [`AccessControlError::AccountNotFound`] - If the indexing is out of
///   bounds.
pub fn get_role_member(e: &Env, role: &Symbol, index: u32) -> Address {
    let key = AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index });

    if let Some(account) = e.storage().persistent().get(&key) {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
        account
    } else {
        panic_with_error!(e, AccessControlError::AccountNotFound)
    }
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
    if let Some(admin_role) = e.storage().persistent().get(&key) {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
        Some(admin_role)
    } else {
        None
    }
}

// ################## CHANGE STATE ##################

/// Sets the overarching admin role.
///
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `admin` - The account to grant the admin privilege.
///
/// **IMPORTANT**: this function lacks authorization checks.
/// It is expected to call this function only in the constructor!
pub fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&AccessControlStorageKey::Admin, &admin);
}

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
/// * refer to [`ensure_if_admin_or_admin_role`] errors.
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
    grant_role_no_auth(e, caller, account, role);
}

/// Low-level function to grant a role to an account without performing
// authorization checks.
/// Creates the role if it does not exist.
/// Returns early if the account already has the role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller.
/// * `account` - The account to grant the role to.
/// * `role` - The role to grant.
///
/// # Events
///
/// * topics - `["role_granted", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods creates significant security
/// risks as it could allow unauthorized role assignments.
pub fn grant_role_no_auth(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
    // Return early if account already has the role
    if has_role(e, account, role).is_some() {
        return;
    }

    let index = add_to_role_enumeration(e, account, role);
    let key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&key, &index);

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
/// * refer to [`ensure_if_admin_or_admin_role`] errors.
/// * refer to [`revoke_role_no_auth`] errors.
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
    revoke_role_no_auth(e, caller, account, role);
}

/// Low-level function to revoke a role from an account without performing
/// authorization checks.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller.
/// * `account` - The account to revoke the role from.
/// * `role` - The role to revoke.
///
/// # Errors
///
/// * [`AccessControlError::AccountNotFound`] - If the `account` doesn't have
///   the role.
/// * refer to [`remove_from_role_enumeration`] errors.
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods creates significant security
/// risks as it could allow unauthorized role revocations.
pub fn revoke_role_no_auth(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
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
/// * [`AccessControlError::AccountNotFound`] - If the `caller` doesn't have the
///   role.
/// * refer to [`remove_from_role_enumeration`] errors.
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
/// * `new_admin` - The account to transfer the admin privileges to.
/// * `live_until_ledger` - The ledger number at which the pending transfer
///   expires. If `live_until_ledger` is `0`, the pending transfer is cancelled.
///   `live_until_ledger` argument is implicitly bounded by the maximum allowed
///   TTL extension for a temporary storage entry and specifying a higher value
///   will cause the code to panic.
///
/// # Errors
///
/// * refer to [`transfer_role`] errors.
///
/// # Events
///
/// * topics - `["admin_transfer_initiated", current_admin: Address]`
/// * data - `[new_admin: Address, live_until_ledger: u32]`
///
/// # Notes
///
/// * Authorization for the current admin is required.
pub fn transfer_admin_role(e: &Env, new_admin: &Address, live_until_ledger: u32) {
    let admin = get_admin(e);
    admin.require_auth();

    transfer_role(e, new_admin, &AccessControlStorageKey::PendingAdmin, live_until_ledger);

    emit_admin_transfer_initiated(e, &admin, new_admin, live_until_ledger);
}

/// Completes the 2-step admin transfer.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * refer to [`accept_transfer`] errors.
///
/// # Events
///
/// * topics - `["admin_transfer_completed", new_admin: Address]`
/// * data - `[previous_admin: Address]`
///
/// # Notes
///
/// * Authorization for the pending admin is required.
pub fn accept_admin_transfer(e: &Env) {
    let previous_admin = get_admin(e);

    let new_admin =
        accept_transfer(e, &AccessControlStorageKey::Admin, &AccessControlStorageKey::PendingAdmin);

    emit_admin_transfer_completed(e, &previous_admin, &new_admin);
}

/// Sets `admin_role` as the admin role for `role`.
/// The admin role for a role controls who can grant and revoke that role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to set the admin for.
/// * `admin_role` - The role that will be the admin.
///
/// # Events
///
/// * topics - `["role_admin_changed", role: Symbol]`
/// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
///
/// # Errors
///
/// * refer to [`get_admin`] errors.
///
/// # Notes
///
/// * Authorization for the current admin is required.
pub fn set_role_admin(e: &Env, role: &Symbol, admin_role: &Symbol) {
    let admin = get_admin(e);
    admin.require_auth();

    set_role_admin_no_auth(e, role, admin_role);
}

/// Low-level function to set the admin role for a specified role without
/// performing authorization checks.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to set the admin for.
/// * `admin_role` - The new admin role to set.
///
/// # Events
///
/// * topics - `["role_admin_changed", role: Symbol]`
/// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods creates significant security
/// risks as it could allow unauthorized admin role assignments.
pub fn set_role_admin_no_auth(e: &Env, role: &Symbol, admin_role: &Symbol) {
    let key = AccessControlStorageKey::RoleAdmin(role.clone());

    // Get previous admin role if exists
    let previous_admin_role =
        e.storage().persistent().get::<_, Symbol>(&key).unwrap_or_else(|| Symbol::new(e, ""));

    e.storage().persistent().set(&key, admin_role);

    emit_role_admin_changed(e, role, &previous_admin_role, admin_role);
}

// ################## LOW-LEVEL HELPERS ##################

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
/// * [`AccessControlError::Unauthorized`] - If the caller is neither the
///   contract admin nor has the admin role.
/// * refer to [`get_admin`] errors.
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
/// * [`AccessControlError::Unauthorized`] - If the caller does not have the
///   specified role.
pub fn ensure_role(e: &Env, caller: &Address, role: &Symbol) {
    if has_role(e, caller, role).is_none() {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }
}

/// Retrieves the admin from storage, enforces authorization,
/// and returns the admin address.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Returns
///
/// The admin address if authorization is successful.
///
/// # Errors
///
/// * refer to [`get_admin`] errors.
pub fn enforce_admin_auth(e: &Env) -> Address {
    let admin = get_admin(e);
    admin.require_auth();
    admin
}

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

    // Store the index for the account in HasRole
    let has_role_key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&has_role_key, &count);

    // Update the count
    e.storage().persistent().set(&count_key, &(count + 1));

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
/// * [`AccessControlError::AccountNotFound`] - If the role has no members or
///   the `account` doesn't have the role.
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
}
