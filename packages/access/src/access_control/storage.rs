use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol, Vec};

use crate::{
    access_control::{
        emit_admin_renounced, emit_admin_transfer_completed, emit_admin_transfer_initiated,
        emit_role_admin_changed, emit_role_granted, emit_role_revoked, AccessControlError,
        MAX_ROLES, ROLE_EXTEND_AMOUNT, ROLE_TTL_THRESHOLD,
    },
    role_transfer::{accept_transfer, transfer_role},
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
    ExistingRoles,                // Vec<Symbol> of all existing roles
    RoleAccounts(RoleAccountKey), // (role, index) -> Address
    HasRole(Address, Symbol),     // (account, role) -> index
    RoleAccountsCount(Symbol),    // role -> count of accounts with the role
    RoleAdmin(Symbol),            // role -> the admin of the role
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

    e.storage().persistent().get(&key).inspect(|_| {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT)
    })
}

/// Returns the admin account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
pub fn get_admin(e: &Env) -> Option<Address> {
    e.storage().instance().get(&AccessControlStorageKey::Admin)
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
/// * [`AccessControlError::IndexOutOfBounds`] - If the indexing is out of
///   bounds.
pub fn get_role_member(e: &Env, role: &Symbol, index: u32) -> Address {
    let key = AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index });

    if let Some(account) = e.storage().persistent().get(&key) {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
        account
    } else {
        panic_with_error!(e, AccessControlError::IndexOutOfBounds)
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

    e.storage().persistent().get(&key).inspect(|_| {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT)
    })
}

/// Returns a vector containing all existing roles.
/// Defaults to empty vector if no roles exist.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Notes
///
/// This function returns all roles that currently have at least one member.
pub fn get_existing_roles(e: &Env) -> Vec<Symbol> {
    let key = AccessControlStorageKey::ExistingRoles;
    if let Some(existing_roles) = e.storage().persistent().get(&key) {
        e.storage().persistent().extend_ttl(&key, ROLE_TTL_THRESHOLD, ROLE_EXTEND_AMOUNT);
        existing_roles
    } else {
        Vec::new(e)
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
/// # Errors
///
/// * [`AccessControlError::AdminAlreadySet`] - If the admin is already set.
///
/// **IMPORTANT**: this function lacks authorization checks.
/// It is expected to call this function only in the constructor!
pub fn set_admin(e: &Env, admin: &Address) {
    // Check if admin is already set
    if e.storage().instance().has(&AccessControlStorageKey::Admin) {
        panic_with_error!(e, AccessControlError::AdminAlreadySet);
    }
    e.storage().instance().set(&AccessControlStorageKey::Admin, &admin);
}

/// Grants a role to an account.
/// Creates the role if it does not exist.
/// Returns early if the account already has the role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to grant the role to.
/// * `role` - The role to grant.
/// * `caller` - The address of the caller, must be the admin or has the
///   `AdminRole` privileges for this role.
///
/// # Errors
///
/// * refer to [`ensure_if_admin_or_admin_role`] errors.
/// * refer to [`grant_role_no_auth`] errors.
///
/// # Events
///
/// * topics - `["role_granted", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
///
/// # Notes
///
/// * Authorization for `caller` is required.
pub fn grant_role(e: &Env, account: &Address, role: &Symbol, caller: &Address) {
    caller.require_auth();
    ensure_if_admin_or_admin_role(e, role, caller);
    grant_role_no_auth(e, account, role, caller);
}

/// Low-level function to grant a role to an account without performing
/// authorization checks.
/// Creates the role if it does not exist.
/// Returns early if the account already has the role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to grant the role to.
/// * `role` - The role to grant.
/// * `caller` - The address of the caller.
///
/// # Errors
///
/// * refer to [`add_to_role_enumeration`] errors.
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
pub fn grant_role_no_auth(e: &Env, account: &Address, role: &Symbol, caller: &Address) {
    // Return early if account already has the role
    if has_role(e, account, role).is_some() {
        return;
    }
    add_to_role_enumeration(e, account, role);

    emit_role_granted(e, role, account, caller);
}

/// Revokes a role from an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to revoke the role from.
/// * `role` - The role to revoke.
/// * `caller` - The address of the caller, must be the admin or has the
///   `AdminRole` privileges for this role.
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
pub fn revoke_role(e: &Env, account: &Address, role: &Symbol, caller: &Address) {
    caller.require_auth();
    ensure_if_admin_or_admin_role(e, role, caller);
    revoke_role_no_auth(e, account, role, caller);
}

/// Low-level function to revoke a role from an account without performing
/// authorization checks.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to revoke the role from.
/// * `role` - The role to revoke.
/// * `caller` - The address of the caller.
///
/// # Errors
///
/// * [`AccessControlError::RoleNotHeld`] - If the `account` doesn't have the
///   role.
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
pub fn revoke_role_no_auth(e: &Env, account: &Address, role: &Symbol, caller: &Address) {
    // Check if account has the role
    if has_role(e, account, role).is_none() {
        panic_with_error!(e, AccessControlError::RoleNotHeld);
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
/// * `role` - The role to renounce.
/// * `caller` - The address of the caller, must be the account that has the
///   role.
///
/// # Errors
///
/// * [`AccessControlError::RoleNotHeld`] - If the `caller` doesn't have the
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
pub fn renounce_role(e: &Env, role: &Symbol, caller: &Address) {
    caller.require_auth();

    // Check if account has the role
    if has_role(e, caller, role).is_none() {
        panic_with_error!(e, AccessControlError::RoleNotHeld);
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
/// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
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
    let admin = enforce_admin_auth(e);

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
/// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
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
    let Some(previous_admin) = get_admin(e) else {
        panic_with_error!(e, AccessControlError::AdminNotSet);
    };

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
/// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
///
/// # Notes
///
/// * Authorization for the current admin is required.
pub fn set_role_admin(e: &Env, role: &Symbol, admin_role: &Symbol) {
    let Some(admin) = get_admin(e) else {
        panic_with_error!(e, AccessControlError::AdminNotSet);
    };
    admin.require_auth();

    set_role_admin_no_auth(e, role, admin_role);
}

/// Allows the current admin to renounce their role, making the contract
/// permanently admin-less. This is useful for decentralization purposes or when
/// the admin role is no longer needed. Once the admin is renounced, it cannot
/// be reinstated.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * [`AccessControlError::AdminNotSet`] - If no admin account is set.
/// * [`AccessControlError::TransferInProgress`] - If there is a pending admin
///   transfer.
///
/// # Events
///
/// * topics - `["admin_renounced", admin: Address]`
/// * data - `[]`
///
/// # Notes
///
/// * Authorization for the current admin is required.
pub fn renounce_admin(e: &Env) {
    let admin = enforce_admin_auth(e);
    let key = AccessControlStorageKey::PendingAdmin;

    if e.storage().temporary().get::<_, Address>(&key).is_some() {
        panic_with_error!(e, AccessControlError::TransferInProgress);
    }

    e.storage().instance().remove(&AccessControlStorageKey::Admin);

    emit_admin_renounced(e, &admin);
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
///
/// # Circular Admin Warning
///
/// **CAUTION**: This function allows the creation of circular admin
/// relationships between roles. For example, it's possible to assign MINT_ADMIN
/// as the admin of MINT_ROLE while also making MINT_ROLE the admin of
/// MINT_ADMIN. Such circular relationships can lead to unintended consequences,
/// including:
///
/// - Race conditions where each role can revoke the other
/// - Potential security vulnerabilities in role management
/// - Confusing governance structures that are difficult to reason about
///
/// When designing your role hierarchy, carefully consider the relationships
/// between roles and avoid creating circular dependencies.
pub fn set_role_admin_no_auth(e: &Env, role: &Symbol, admin_role: &Symbol) {
    let key = AccessControlStorageKey::RoleAdmin(role.clone());

    // Get previous admin role if exists
    let previous_admin_role =
        e.storage().persistent().get::<_, Symbol>(&key).unwrap_or_else(|| Symbol::new(e, ""));

    e.storage().persistent().set(&key, admin_role);

    emit_role_admin_changed(e, role, &previous_admin_role, admin_role);
}

/// Removes the admin role for a specified role without performing authorization
/// checks.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to remove the admin for.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - In admin functions that implement their own authorization logic
/// - When cleaning up unused roles
pub fn remove_role_admin_no_auth(e: &Env, role: &Symbol) {
    let key = AccessControlStorageKey::RoleAdmin(role.clone());

    // Check if the key exists before attempting to remove
    if e.storage().persistent().has(&key) {
        e.storage().persistent().remove(&key);
    } else {
        panic_with_error!(e, AccessControlError::AdminRoleNotFound);
    }
}

/// Removes the role accounts count for a specified role without performing
/// authorization checks.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to remove the accounts count for.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - In admin functions that implement their own authorization logic
/// - When cleaning up unused roles with zero members
pub fn remove_role_accounts_count_no_auth(e: &Env, role: &Symbol) {
    let count_key = AccessControlStorageKey::RoleAccountsCount(role.clone());

    // Check if the key exists and has a zero count before removing
    if let Some(count) = e.storage().persistent().get::<_, u32>(&count_key) {
        if count == 0 {
            e.storage().persistent().remove(&count_key);
        } else {
            panic_with_error!(e, AccessControlError::RoleCountIsNotZero);
        }
    } else {
        panic_with_error!(e, AccessControlError::RoleNotFound);
    }
}

// ################## LOW-LEVEL HELPERS ##################

/// Ensures that the caller is either the contract admin or has the admin role
/// for the specified role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role to check admin privileges for.
/// * `caller` - The address of the caller to check permissions for.
///
/// # Errors
///
/// * [`AccessControlError::Unauthorized`] - If the caller is neither the
///   contract admin nor has the admin role.
pub fn ensure_if_admin_or_admin_role(e: &Env, role: &Symbol, caller: &Address) {
    // Check if caller is contract admin (if one is set)
    let is_admin = match get_admin(e) {
        Some(admin) => caller == &admin,
        None => false,
    };

    // Check if caller has admin role for the specified role
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
/// * `role` - The role to check for.
/// * `caller` - The address of the caller to check the role for.
///
/// # Errors
///
/// * [`AccessControlError::Unauthorized`] - If the caller does not have the
///   specified role.
pub fn ensure_role(e: &Env, role: &Symbol, caller: &Address) {
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
/// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
pub fn enforce_admin_auth(e: &Env) -> Address {
    let Some(admin) = get_admin(e) else {
        panic_with_error!(e, AccessControlError::AdminNotSet);
    };

    admin.require_auth();
    admin
}

/// Adds an account to role enumeration.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `account` - The account to add to the role.
/// * `role` - The role to add the account to.
///
/// # Errors
///
/// * [`AccessControlError::MaxRolesExceeded`] - If adding a new role would
///   exceed the maximum allowed number of roles.
pub fn add_to_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
    // Get the current count of accounts with this role
    let count_key = AccessControlStorageKey::RoleAccountsCount(role.clone());
    let count = e.storage().persistent().get(&count_key).unwrap_or(0);

    // If this is the first account with this role, add the role to ExistingRoles
    if count == 0 {
        let mut existing_roles = get_existing_roles(e);

        // Check if we've reached the maximum number of roles
        if existing_roles.len() == MAX_ROLES {
            panic_with_error!(e, AccessControlError::MaxRolesExceeded);
        }

        existing_roles.push_back(role.clone());
        e.storage().persistent().set(&AccessControlStorageKey::ExistingRoles, &existing_roles);
    }

    // Add the account to the enumeration
    let new_key =
        AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index: count });
    e.storage().persistent().set(&new_key, account);

    // Store the index for the account in HasRole
    let has_role_key = AccessControlStorageKey::HasRole(account.clone(), role.clone());
    e.storage().persistent().set(&has_role_key, &count);

    // Update the count
    e.storage().persistent().set(&count_key, &(count + 1));
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
/// * [`AccessControlError::RoleIsEmpty`] - If the role has no members.
/// * [`AccessControlError::RoleNotHeld`] - If the `account` doesn't have the
///   role.
pub fn remove_from_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
    // Get the current count of accounts with this role
    let count_key = AccessControlStorageKey::RoleAccountsCount(role.clone());
    let count = e.storage().persistent().get(&count_key).unwrap_or(0);
    if count == 0 {
        panic_with_error!(e, AccessControlError::RoleIsEmpty);
    }

    // Get the index of the account to remove
    let to_be_removed_has_role_key =
        AccessControlStorageKey::HasRole(account.clone(), role.clone());
    let to_be_removed_index = e
        .storage()
        .persistent()
        .get::<_, u32>(&to_be_removed_has_role_key)
        .unwrap_or_else(|| panic_with_error!(e, AccessControlError::RoleNotHeld));

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

    // If this was the last account with this role, remove the role from
    // ExistingRoles
    if last_index == 0 {
        let mut existing_roles = get_existing_roles(e);

        // Find and remove the role
        if let Some(pos) = existing_roles.iter().position(|r| r == role.clone()) {
            existing_roles.remove(pos as u32);
            e.storage().persistent().set(&AccessControlStorageKey::ExistingRoles, &existing_roles);
        }
    }
}
