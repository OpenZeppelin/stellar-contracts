//! Access control module for Soroban contracts
//!
//! This module provides functionality to manage role-based access control in Soroban contracts.
//!
//! # Usage
//!
//! One can create new methods for their contract and specify which roles can do what.
//! For example, one could create a role `mint_admins` and specify a 2nd level administration.
//! This group (mint_admins) may have access to `revoke_mint_role` and `grant_mint_role` methods.
//!
//! In order to do that:
//! 1. the admin will create the `minter_admin` role and specify the accounts for that with [`grant_role()`] function.
//!
//! Then, the new methods can be implemented for the contract may look like this:
//!
//! ```rust
//! #[has_role(caller, "minter_admin")]
//! pub fn grant_mint_role(e: &Env, caller: Address) {
//!     ...
//! }
//! ```
//!
//! If multi-admin setup is wanted, it can be achieved in a similar way by creating a new admin role
//! and assigning accounts to it.

#![no_std]

mod access_control;
mod storage;

use soroban_sdk::{contracttype, Address, Env, Symbol};

pub use crate::{
    access_control::{AccessControl, AccessControlError},
    storage::{
        accept_admin_transfer, add_to_role_enumeration, cancel_transfer_admin_role, get_admin,
        get_role_admin, get_role_member, get_role_member_count, grant_role, has_role,
        remove_from_role_enumeration, revoke_role, set_role_admin, transfer_admin_role,
    },
};

mod test;
