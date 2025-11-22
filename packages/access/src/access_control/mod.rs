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

#[cfg(test)]
mod test;

#[cfg(feature = "certora")]
pub mod specs;

use soroban_sdk::{contracterror, Address, Env, Symbol};

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

pub use crate::access_control::storage::{
    accept_admin_transfer, add_to_role_enumeration, enforce_admin_auth,
    ensure_if_admin_or_admin_role, ensure_role, get_admin, get_role_admin, get_role_member,
    get_role_member_count, grant_role, grant_role_no_auth, has_role, remove_from_role_enumeration,
    remove_role_accounts_count_no_auth, remove_role_admin_no_auth, renounce_admin, renounce_role,
    revoke_role, revoke_role_no_auth, set_admin, set_role_admin, set_role_admin_no_auth,
    transfer_admin_role, AccessControlStorageKey,
};


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
    /// * [`AccessControlError::IndexOutOfBounds`] - If the index is out of
    ///   bounds for the role's member list.
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
    /// * [`AccessControlError::Unauthorized`] - If the caller does not have
    ///   enough privileges.
    ///
    /// # Events
    ///
    /// * topics - `["role_granted", role: Symbol, account: Address]`
    /// * data - `[caller: Address]`
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
    /// * [`AccessControlError::Unauthorized`] - If the `caller` does not have
    ///   enough privileges.
    /// * [`AccessControlError::RoleNotHeld`] - If the `account` doesn't have
    ///   the role.
    /// * [`AccessControlError::RoleIsEmpty`] - If the role has no members.
    ///
    /// # Events
    ///
    /// * topics - `["role_revoked", role: Symbol, account: Address]`
    /// * data - `[caller: Address]`
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
    /// * [`AccessControlError::RoleNotHeld`] - If the `caller` doesn't have the
    ///   role.
    /// * [`AccessControlError::RoleIsEmpty`] - If the role has no members.
    ///
    /// # Events
    ///
    /// * topics - `["role_revoked", role: Symbol, account: Address]`
    /// * data - `[caller: Address]`
    fn renounce_role(e: &Env, caller: Address, role: Symbol);

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
    /// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
    ///
    /// # Events
    ///
    /// * topics - `["admin_transfer_initiated", current_admin: Address]`
    /// * data - `[new_admin: Address, live_until_ledger: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for the current admin is required.
    fn transfer_admin_role(e: &Env, new_admin: Address, live_until_ledger: u32);

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
    /// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
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
    /// * [`AccessControlError::AdminNotSet`] - If admin account is not set.
    ///
    /// # Notes
    ///
    /// * Authorization for the current admin is required.
    fn set_role_admin(e: &Env, role: Symbol, admin_role: Symbol);

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
    /// * [`AccessControlError::AdminNotSet`] - If no admin account is set.
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
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccessControlError {
    Unauthorized = 2000,
    AdminNotSet = 2001,
    IndexOutOfBounds = 2002,
    AdminRoleNotFound = 2003,
    RoleCountIsNotZero = 2004,
    RoleNotFound = 2005,
    AdminAlreadySet = 2006,
    RoleNotHeld = 2007,
    RoleIsEmpty = 2008,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const ROLE_EXTEND_AMOUNT: u32 = 90 * DAY_IN_LEDGERS;
pub const ROLE_TTL_THRESHOLD: u32 = ROLE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when a role is granted.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleGranted {
    #[topic]
    pub role: Symbol,
    #[topic]
    pub account: Address,
    pub caller: Address,
}

/// Emits an event when a role is granted to an account.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role that was granted.
/// * `account` - The account that received the role.
/// * `caller` - The account that granted the role.
#[cfg(not(feature = "certora"))]
pub fn emit_role_granted(e: &Env, role: &Symbol, account: &Address, caller: &Address) {
    RoleGranted { role: role.clone(), account: account.clone(), caller: caller.clone() }.publish(e);
}

/// Event emitted when a role is revoked.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleRevoked {
    #[topic]
    pub role: Symbol,
    #[topic]
    pub account: Address,
    pub caller: Address,
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
#[cfg(not(feature = "certora"))]
pub fn emit_role_revoked(e: &Env, role: &Symbol, account: &Address, caller: &Address) {
    RoleRevoked { role: role.clone(), account: account.clone(), caller: caller.clone() }.publish(e);
}

/// Event emitted when a role admin is changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleAdminChanged {
    #[topic]
    pub role: Symbol,
    pub previous_admin_role: Symbol,
    pub new_admin_role: Symbol,
}

/// Emits an event when the admin role for a role changes.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `role` - The role whose admin is changing.
/// * `previous_admin_role` - The previous admin role.
/// * `new_admin_role` - The new admin role.
#[cfg(not(feature = "certora"))]
pub fn emit_role_admin_changed(
    e: &Env,
    role: &Symbol,
    previous_admin_role: &Symbol,
    new_admin_role: &Symbol,
) {
    RoleAdminChanged {
        role: role.clone(),
        previous_admin_role: previous_admin_role.clone(),
        new_admin_role: new_admin_role.clone(),
    }
    .publish(e);
}

/// Event emitted when an admin transfer is initiated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferInitiated {
    #[topic]
    pub current_admin: Address,
    pub new_admin: Address,
    pub live_until_ledger: u32,
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
#[cfg(not(feature = "certora"))]
pub fn emit_admin_transfer_initiated(
    e: &Env,
    current_admin: &Address,
    new_admin: &Address,
    live_until_ledger: u32,
) {
    AdminTransferInitiated {
        current_admin: current_admin.clone(),
        new_admin: new_admin.clone(),
        live_until_ledger,
    }
    .publish(e);
}

/// Event emitted when an admin transfer is completed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferCompleted {
    #[topic]
    pub new_admin: Address,
    pub previous_admin: Address,
}

/// Emits an event when an admin transfer is completed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `previous_admin` - The previous admin.
/// * `new_admin` - The new admin who accepted the transfer.
#[cfg(not(feature = "certora"))]
pub fn emit_admin_transfer_completed(e: &Env, previous_admin: &Address, new_admin: &Address) {
    AdminTransferCompleted { new_admin: new_admin.clone(), previous_admin: previous_admin.clone() }
        .publish(e);
}

/// Event emitted when the admin role is renounced.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRenounced {
    #[topic]
    pub admin: Address,
}

/// Emits an event when the admin role is renounced.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `admin` - The admin that renounced the role.
#[cfg(not(feature = "certora"))]
pub fn emit_admin_renounced(e: &Env, admin: &Address) {
    AdminRenounced { admin: admin.clone() }.publish(e);
}