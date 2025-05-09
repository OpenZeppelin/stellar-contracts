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
//! Each role can have an `admin role` specified for it. For example, if you
//! create 2 roles:
//! - minter
//! - mint_admins
//!
//! You can assign the role `mint_admins` as the admin role of the `minter` role
//! group. And this will allow accounts with `mint_admins` role, to grant and
//! revoke the roles of `minter` roles.
//!
//! One can create as many roles as they want, and create a chain of command
//! structure if they want to with this approach.
//!
//! If you need even more granular control over which roles can do what, you can
//! introduce your own business logic, and annotate it with our macro:
//!
//! ```rust
//! #[has_role(caller, "minter_admin")]
//! pub fn do_very_specific_stuff_that_is_probably_not_needed_but_we_decided_to_overkill_it_nevertheless(e: &Env, caller: Address) {
//!     ...
//! }
//! ```

#![no_std]

mod access_control;
mod storage;

pub use crate::{
    access_control::{
        emit_admin_transfer, emit_admin_transfer_completed, emit_role_admin_changed,
        emit_role_granted, emit_role_revoked, AccessControl, AccessControlError,
    },
    storage::{
        accept_admin_transfer, add_to_role_enumeration, ensure_role, get_admin, get_role_admin,
        get_role_member, get_role_member_count, grant_role, has_role, remove_from_role_enumeration,
        renounce_role, revoke_role, set_role_admin, transfer_admin_role, AccessControlStorageKey,
    },
};

mod test;
