use soroban_sdk::{contracterror, Address, Env, Symbol};

pub trait AccessControl {
    /// Returns `Some(index)` if the account has the specified role,
    /// where `index` is the position of the account for that role,
    /// and can be used to query [`AccessControl::get_role_member()`].
    /// Returns `None` if the account does not have the specified role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `account` - The account to check.
    /// * `role` - The role to check for.
    fn has_role(e: &Env, account: Address, role: Symbol) -> Option<u32>;

    /// Returns the total number of accounts that have the specified role.
    /// If the role does not exist, returns 0.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to get the count for.
    fn get_role_member_count(e: &Env, role: Symbol) -> u32;

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
    /// * [`AccessControlError::OutOfBounds`] - If the indexing is out of bounds.
    fn get_role_member(e: &Env, role: Symbol, index: u32) -> Address;

    /// Returns the admin role for a specific role.
    /// If no admin role is explicitly set, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to query the admin role for.
    fn get_role_admin(e: &Env, role: Symbol) -> Option<Symbol>;

    /// Returns the admin account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Errors
    ///
    /// * `AccessControlError::AccountNotFound` - If no admin account is set.
    fn get_admin(e: &Env) -> Address;

    /// Grants a role to an account.
    /// Creates the role if it does not exist.
    /// Returns early if the account already has the role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be the admin or has the
    ///   `RoleAdmin` for the `role`.
    /// * `account` - The account to grant the role to.
    /// * `role` - The role to grant.
    ///
    /// # Errors
    ///
    /// * `AccessControlError::Unauthorized` - If the caller does not have
    ///   enough privileges.
    ///
    /// # Events
    ///
    /// * topics - `["role_granted", role: Symbol, account: Address]`
    /// * data - `[sender: Address]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    /// The caller must be the admin or has the `RoleAdmin` for the `role`.
    fn grant_role(e: &Env, caller: Address, account: Address, role: Symbol);

    /// Revokes a role from an account.
    /// To revoke your own role, please use [`AccessControl::renounce_role()`]
    /// instead.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be the admin or has the
    ///   `RoleAdmin` for the `role`.
    /// * `account` - The account to revoke the role from.
    /// * `role` - The role to revoke.
    ///
    /// # Errors
    ///
    /// * `AccessControlError::Unauthorized` - If the `caller` does not have
    ///   enough privileges.
    /// * `AccessControlError::AccountNotFound` - If the `account` doesn't have
    ///   the role.
    ///
    /// # Events
    ///
    /// * topics - `["role_revoked", role: Symbol, account: Address]`
    /// * data - `[sender: Address]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    /// The caller must be the admin or has the `RoleAdmin` for the `role`.
    fn revoke_role(e: &Env, caller: Address, account: Address, role: Symbol);

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
    /// * `AccessControlError::AccountNotFound` - If the `caller` doesn't have
    ///   the role.
    ///
    /// # Events
    ///
    /// * topics - `["role_revoked", role: Symbol, account: Address]`
    /// * data - `[sender: Address]`
    fn renounce_role(e: &Env, caller: Address, role: Symbol);

    /// Initiates the admin role transfer.
    /// Admin privileges for the current admin are not revoked until the
    /// recipient accepts the transfer.
    /// Overrides the previous pending transfer if there is one.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be the admin.
    /// * `new_admin` - The account to transfer the admin privileges to.
    /// * `live_until_ledger` - The ledger number at which the pending transfer
    ///   expires. If `live_until_ledger` is `0`, the pending transfer is
    ///   cancelled. `live_until_ledger` argument is implicitly bounded by the
    ///   maximum allowed TTL extension for a temporary storage entry and
    ///   specifying a higher value will cause the code to panic.
    ///
    /// # Errors
    ///
    /// * `AccessControlError::Unauthorized` - If the `caller` is not the admin.
    /// * `AccessControlError::NoPendingAdminTransfer` - If tried to cancel the
    ///   pending admin transfer when there is no pending admin transfer.
    ///
    /// # Events
    ///
    /// * topics - `["admin_transfer_started", current_admin: Address]`
    /// * data - `[new_admin: Address, live_until_ledger: u32]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    fn transfer_admin_role(e: &Env, caller: Address, new_admin: Address, live_until_ledger: u32);

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
    /// * `AccessControlError::Unauthorized` - If the `caller` is not the
    ///   pending admin.
    ///
    /// # Events
    ///
    /// * topics - `["admin_transfer_completed", new_admin: Address]`
    /// * data - `[previous_admin: Address]`
    fn accept_admin_transfer(e: &Env, caller: Address);

    /// Sets `admin_role` as the admin role of `role`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be the admin.
    /// * `role` - The role to set the admin for.
    /// * `admin_role` - The new admin role.
    ///
    /// # Errors
    ///
    /// * `AccessControlError::Unauthorized` - If the `caller` is not the admin.
    ///
    /// # Events
    ///
    /// * topics - `["role_admin_changed", role: Symbol]`
    /// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    fn set_role_admin(e: &Env, caller: Address, role: Symbol, admin_role: Symbol);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccessControlError {
    Unauthorized = 120,
    RoleNotFound = 121,
    AccountNotFound = 122,
    NoPendingAdminTransfer = 123,
    OutOfBounds = 124,
    InvalidLiveUntilLedger = 125,
}

// ################## EVENTS ##################

/// Emits an event when a role is granted to an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role that was granted.
/// * `account` - The account that received the role.
/// * `sender` - The account that granted the role.
///
/// # Events
///
/// * topics - `["role_granted", role: Symbol, account: Address]`
/// * data - `[sender: Address]`
pub fn emit_role_granted(e: &Env, role: &Symbol, account: &Address, sender: &Address) {
    let topics = (Symbol::new(e, "role_granted"), role, account);
    e.events().publish(topics, sender);
}

/// Emits an event when a role is revoked from an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role that was revoked.
/// * `account` - The account that lost the role.
/// * `sender` - The account that revoked the role (either the admin or the
///   account itself).
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[sender: Address]`
pub fn emit_role_revoked(e: &Env, role: &Symbol, account: &Address, sender: &Address) {
    let topics = (Symbol::new(e, "role_revoked"), role, account);
    e.events().publish(topics, sender);
}

/// Emits an event when the admin role for a role changes.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role whose admin is changing.
/// * `previous_admin_role` - The previous admin role.
/// * `new_admin_role` - The new admin role.
///
/// # Events
///
/// * topics - `["role_admin_changed", role: Symbol]`
/// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
pub fn emit_role_admin_changed(
    e: &Env,
    role: &Symbol,
    previous_admin_role: &Symbol,
    new_admin_role: &Symbol,
) {
    let topics = (Symbol::new(e, "role_admin_changed"), role);
    e.events().publish(topics, (previous_admin_role, new_admin_role));
}

/// Emits an event when an admin transfer is initiated.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `current_admin` - The current admin initiating the transfer.
/// * `new_admin` - The proposed new admin.
/// * `live_until_ledger` - The ledger number at which the pending transfer will
///   expire. If this value is `0`, it means the pending transfer is cancelled.
///
/// # Events
///
/// * topics - `["admin_transfer", current_admin: Address]`
/// * data - `[new_admin: Address, live_until_ledger: u32]`
pub fn emit_admin_transfer(
    e: &Env,
    current_admin: &Address,
    new_admin: &Address,
    live_until_ledger: u32,
) {
    let topics = (Symbol::new(e, "admin_transfer"), current_admin);
    e.events().publish(topics, (new_admin, live_until_ledger));
}

/// Emits an event when an admin transfer is completed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `previous_admin` - The previous admin.
/// * `new_admin` - The new admin who accepted the transfer.
///
/// # Events
///
/// * topics - `["admin_transfer_completed", new_admin: Address]`
/// * data - `[previous_admin: Address]`
pub fn emit_admin_transfer_completed(e: &Env, previous_admin: &Address, new_admin: &Address) {
    let topics = (Symbol::new(e, "admin_transfer_completed"), new_admin);
    e.events().publish(topics, previous_admin);
}
