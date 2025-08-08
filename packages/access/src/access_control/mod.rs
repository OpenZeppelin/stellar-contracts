//! Access control module for Soroban contracts
//!
//! This module provides functionality to manage role-based access control in
//! Soroban contracts.
//!
//! # Usage
//!
//! There is a single overarching admin, and the admin has enough privileges to
//! call any function given in the [`AccessControl`] trait.
//!
//! This `admin` must be set in the constructor of the contract. Else, none of
//! the methods exposed by this module will work. You can follow the
//! `nft-access-control` example.
//!
//! ## Admin Transfers
//!
//! Transferring the top-level admin is a critical action, and as such, it is
//! implemented as a **two-step process** to prevent accidental or malicious
//! takeovers:
//!
//! 1. The current admin **initiates** the transfer by specifying the
//!    `new_admin` and a `live_until_ledger`, which defines the expiration time
//!    for the offer.
//! 2. The designated `new_admin` must **explicitly accept** the transfer to
//!    complete it.
//!
//! Until the transfer is accepted, the original admin retains full control, and
//! the transfer can be overridden or canceled by initiating a new one or using
//! a `live_until_ledger` of `0`.
//!
//! This handshake mechanism ensures that the recipient is aware and willing to
//! assume responsibility, providing a robust safeguard in governance-sensitive
//! deployments.
//!
//! ## Role Hierarchy
//!
//! Each role can have an "admin role" specified for it. For example, if you
//! create two roles: `minter` and `minter_admin`, you can assign
//! `minter_admin` as the admin role for the `minter` role. This will allow
//! to accounts with `minter_admin` role to grant/revoke the `minter` role
//! to other accounts.
//!
//! One can create as many roles as they want, and create a chain of command
//! structure if they want to go with this approach.
//!
//! If you need even more granular control over which roles can do what, you can
//! introduce your own business logic, and annotate it with our macro:
//!
//! ```rust
//! #[has_role(caller, "minter_admin")]
//! pub fn custom_sensitive_logic(e: &Env, caller: Address) {
//!     ...
//! }
//! ```
//!
//! ### ⚠️ Warning: Circular Admin Relationships
//!
//! When designing your role hierarchy, be careful to avoid creating circular
//! admin relationships. For example, it's possible but not recommended to
//! assign `MINT_ADMIN` as the admin of `MINT_ROLE` while also making
//! `MINT_ROLE` the admin of `MINT_ADMIN`. Such circular relationships can lead
//! to unintended consequences, including:
//!
//! - Race conditions where each role can revoke the other
//! - Potential security vulnerabilities in role management
//! - Confusing governance structures that are difficult to reason about
//!
//! ## Enumeration of Roles
//!
//! In this access control system, roles don't exist as standalone entities.
//! Instead, the system stores account-role pairs in storage with additional
//! enumeration logic:
//!
//! - When a role is granted to an account, the account-role pair is stored and
//!   added to enumeration storage (RoleAccountsCount and RoleAccounts).
//! - When a role is revoked from an account, the account-role pair is removed
//!   from storage and from enumeration.
//! - If all accounts are removed from a role, the helper storage items for that
//!   role become empty or 0, but the entries themselves remain.
//!
//! This means that the question of whether a role can "exist" with 0 accounts
//! is technically invalid, because roles only exist through their relationships
//! with accounts. When checking if a role has any accounts via
//! `get_role_member_count`, it returns 0 in two cases:
//!
//! 1. When accounts were assigned to a role but later all were removed.
//! 2. When a role never existed in the first place.

mod storage;

mod test;

use soroban_sdk::{assert_with_error, contracterror, contracttrait, Address, Env, Symbol};

pub use crate::access_control::storage::{AccessControlStorageKey, AccessController};

#[contracttrait(add_impl_type = true)]
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
    fn has_role(e: &Env, account: &soroban_sdk::Address, role: &soroban_sdk::Symbol)
        -> Option<u32>;

    /// Returns the total number of accounts that have the specified role.
    /// If the role does not exist, returns 0.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to get the count for.
    fn get_role_member_count(e: &Env, role: &soroban_sdk::Symbol) -> u32;

    /// Returns the account at the specified index for a given role.
    ///
    /// We do not provide a function to get all the members of a role,
    /// since that would be unbounded. If you need to enumerate all the
    /// members of a role, you can use
    /// [`AccessControl::get_role_member_count()`] to get the total number
    /// of members and then use [`AccessControl::get_role_member()`] to get
    /// each member one by one.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to query.
    /// * `index` - The index of the account to retrieve.
    ///
    /// # Errors
    ///
    /// * [`AccessControllerror::IndexOutOfBounds`] - If the index is out of
    ///   bounds for the role's member list.
    fn get_role_member(e: &Env, role: &soroban_sdk::Symbol, index: u32) -> Address;

    /// Returns the admin role for a specific role.
    /// If no admin role is explicitly set, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to query the admin role for.
    fn get_role_admin(e: &Env, role: &soroban_sdk::Symbol) -> Option<soroban_sdk::Symbol>;

    /// Returns the admin account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn get_admin(e: &Env) -> Option<Address>;

    /// Grants a role to an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be the admin or have the
    ///   `RoleAdmin` for the `role`.
    /// * `account` - The account to grant the role to.
    /// * `role` - The role to grant.
    ///
    /// # Errors
    ///
    /// * [`AccessControllerror::Unauthorized`] - If the caller does not have
    ///   enough privileges.
    ///
    /// # Events
    ///
    /// * topics - `["role_granted", role: Symbol, account: Address]`
    /// * data - `[caller: Address]`
    fn grant_role(e: &Env, caller: &Address, account: &Address, role: &soroban_sdk::Symbol);

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
    /// * [`AccessControllerror::Unauthorized`] - If the `caller` does not have
    ///   enough privileges.
    /// * [`AccessControllerror::RoleNotHeld`] - If the `account` doesn't have
    ///   the role.
    /// * [`AccessControllerror::RoleIsEmpty`] - If the role has no members.
    ///
    /// # Events
    ///
    /// * topics - `["role_revoked", role: Symbol, account: Address]`
    /// * data - `[caller: Address]`
    fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &soroban_sdk::Symbol);

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
    /// * [`AccessControllerror::RoleNotHeld`] - If the `caller` doesn't have the
    ///   role.
    /// * [`AccessControllerror::RoleIsEmpty`] - If the role has no members.
    ///
    /// # Events
    ///
    /// * topics - `["role_revoked", role: Symbol, account: Address]`
    /// * data - `[caller: Address]`
    fn renounce_role(e: &Env, caller: &Address, role: &soroban_sdk::Symbol);

    /// Initiates the admin role transfer.
    /// Admin privileges for the current admin are not revoked until the
    /// recipient accepts the transfer.
    /// Overrides the previous pending transfer if there is one.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_admin` - The account to transfer the admin privileges to.
    /// * `live_until_ledger` - The ledger number at which the pending transfer
    ///   expires. If `live_until_ledger` is `0`, the pending transfer is
    ///   cancelled. `live_until_ledger` argument is implicitly bounded by the
    ///   maximum allowed TTL extension for a temporary storage entry and
    ///   specifying a higher value will cause the code to panic.
    ///
    /// # Errors
    ///
    /// * [`crate::role_transfer::RoleTransferError::NoPendingTransfer`] - If
    ///   trying to cancel a transfer that doesn't exist.
    /// * [`crate::role_transfer::RoleTransferError::InvalidLiveUntilLedger`] -
    ///   If the specified ledger is in the past.
    /// * [`crate::role_transfer::RoleTransferError::InvalidPendingAccount`] -
    ///   If the specified pending account is not the same as the provided `new`
    ///   address.
    /// * [`AccessControllerror::AdminNotSet`] - If admin account is not set.
    ///
    /// # Events
    ///
    /// * topics - `["admin_transfer_initiated", current_admin: Address]`
    /// * data - `[new_admin: Address, live_until_ledger: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for the current admin is required.
    fn transfer_admin_role(e: &Env, new_admin: &Address, live_until_ledger: u32);

    /// Completes the 2-step admin transfer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Events
    ///
    /// * topics - `["admin_transfer_completed", new_admin: Address]`
    /// * data - `[previous_admin: Address]`
    ///
    /// # Errors
    ///
    /// * [`crate::role_transfer::RoleTransferError::NoPendingTransfer`] - If
    ///   there is no pending transfer to accept.
    /// * [`AccessControllerror::AdminNotSet`] - If admin account is not set.
    fn accept_admin_transfer(e: &Env);

    /// Sets `admin_role` as the admin role of `role`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to set the admin for.
    /// * `admin_role` - The new admin role.
    ///
    /// # Events
    ///
    /// * topics - `["role_admin_changed", role: Symbol]`
    /// * data - `[previous_admin_role: Symbol, new_admin_role: Symbol]`
    ///
    /// # Errors
    ///
    /// * [`AccessControllerror::AdminNotSet`] - If admin account is not set.
    ///
    /// # Notes
    ///
    /// * Authorization for the current admin is required.
    fn set_role_admin(e: &Env, role: &soroban_sdk::Symbol, admin_role: &soroban_sdk::Symbol);

    /// Allows the current admin to renounce their role, making the contract
    /// permanently admin-less. This is useful for decentralization purposes
    /// or when the admin role is no longer needed. Once the admin is
    /// renounced, it cannot be reinstated.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`AccessControllerror::AdminNotSet`] - If no admin account is set.
    ///
    /// # Events
    ///
    /// * topics - `["admin_renounced", admin: Address]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// * Authorization for the current admin is required.
    fn renounce_admin(e: &Env);

    #[internal]
    fn init_admin(e: &Env, admin: &soroban_sdk::Address);

    #[internal]
    fn enforce_admin_auth(e: &Env) {
        let Some(admin) = Self::get_admin(e) else {
            soroban_sdk::panic_with_error!(e, AccessControllerror::AdminNotSet);
        };
        admin.require_auth();
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
    /// * [`AccessControllerror::Unauthorized`] - If the caller does not have the
    ///   specified role.
    #[internal]
    fn ensure_role(e: &Env, caller: &soroban_sdk::Address, role: &soroban_sdk::Symbol) {
        if Self::has_role(e, caller, role).is_none() {
            soroban_sdk::panic_with_error!(e, AccessControllerror::Unauthorized);
        }
    }

    #[internal]
    fn ensure_if_admin_or_admin_role(
        e: &Env,
        caller: &soroban_sdk::Address,
        role: &soroban_sdk::Symbol,
    ) {
        // Check if caller is contract admin (if one is set)
        let is_admin = match Self::get_admin(e) {
            Some(admin) => caller == &admin,
            None => false,
        };

        // Check if caller has admin role for the specified role
        let is_admin_role = match Self::get_role_admin(e, role) {
            Some(admin_role) => Self::has_role(e, caller, &admin_role).is_some(),
            None => false,
        };

        if !is_admin && !is_admin_role {
            soroban_sdk::panic_with_error!(e, AccessControllerror::Unauthorized);
        }
    }

    #[internal]
    fn grant_role_no_auth(e: &Env, caller: &Address, account: &Address, role: &Symbol);

    #[internal]
    fn remove_role_accounts_count_no_auth(e: &Env, role: &Symbol);

    /// Removes the admin role for a specified role without performing
    /// authorization checks.
    ///
    /// # Arguments
    ///
    /// * `role` - The role to remove the admin for.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used:
    /// - In admin functions that implement their own authorization logic
    /// - When cleaning up unused roles
    #[internal]
    fn remove_role_admin_no_auth(e: &Env, role: &Symbol);

    #[internal]
    fn assert_has_any_role(e: &Env, caller: &Address, roles: &[&str]) {
        assert_with_error!(
            e,
            roles.iter().any(|role| Self::has_role(
                e,
                caller,
                &soroban_sdk::Symbol::new(e, role)).is_some()
            ),
            AccessControllerror::RoleNotHeld
        );
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccessControllerror {
    Unauthorized = 1210,
    AdminNotSet = 1211,
    IndexOutOfBounds = 1212,
    AdminRoleNotFound = 1213,
    RoleCountIsNotZero = 1214,
    RoleNotFound = 1215,
    AdminAlreadySet = 1216,
    RoleNotHeld = 1217,
    RoleIsEmpty = 1218,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const ROLE_EXTEND_AMOUNT: u32 = 90 * DAY_IN_LEDGERS;
pub const ROLE_TTL_THRESHOLD: u32 = ROLE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Emits an event when a role is granted to an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role that was granted.
/// * `account` - The account that received the role.
/// * `caller` - The account that granted the role.
///
/// # Events
///
/// * topics - `["role_granted", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
pub fn emit_role_granted(e: &Env, role: &Symbol, account: &Address, caller: &Address) {
    let topics = (Symbol::new(e, "role_granted"), role, account);
    e.events().publish(topics, caller);
}

/// Emits an event when a role is revoked from an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role that was revoked.
/// * `account` - The account that lost the role.
/// * `caller` - The account that revoked the role (either the admin or the
///   account itself).
///
/// # Events
///
/// * topics - `["role_revoked", role: Symbol, account: Address]`
/// * data - `[caller: Address]`
pub fn emit_role_revoked(e: &Env, role: &Symbol, account: &Address, caller: &Address) {
    let topics = (Symbol::new(e, "role_revoked"), role, account);
    e.events().publish(topics, caller);
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
/// * topics - `["admin_transfer_initiated", current_admin: Address]`
/// * data - `[new_admin: Address, live_until_ledger: u32]`
pub fn emit_admin_transfer_initiated(
    e: &Env,
    current_admin: &Address,
    new_admin: &Address,
    live_until_ledger: u32,
) {
    let topics = (Symbol::new(e, "admin_transfer_initiated"), current_admin);
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

/// Emits an event when the admin role is renounced.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `admin` - The admin that renounced the role.
///
/// # Events
///
/// * topics - `["admin_renounced", admin: Address]`
/// * data - `[]`
pub fn emit_admin_renounced(e: &Env, admin: &Address) {
    let topics = (Symbol::new(e, "admin_renounced"), admin);
    e.events().publish(topics, ());
}
