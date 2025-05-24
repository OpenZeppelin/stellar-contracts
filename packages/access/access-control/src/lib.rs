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
//! structure if they want to with this approach.
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

#![no_std]

mod access_control;
mod storage;

pub use crate::{
    access_control::{
        emit_admin_transfer_completed, emit_admin_transfer_initiated, emit_role_admin_changed,
        emit_role_granted, emit_role_revoked, AccessControl, AccessControlError,
    },
    storage::{
        accept_admin_transfer, add_to_role_enumeration, enforce_admin_auth,
        ensure_if_admin_or_admin_role, ensure_role, get_admin, get_role_admin, get_role_member,
        get_role_member_count, grant_role, grant_role_without_auth, has_role,
        remove_from_role_enumeration, renounce_role, revoke_role, set_admin, set_role_admin,
        set_role_admin_without_auth, transfer_admin_role, AccessControlStorageKey,
    },
};

mod test;
