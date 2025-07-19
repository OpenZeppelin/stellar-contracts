//! # Ownable Contract Module.
//!
//! This module introduces a simple access control mechanism where a contract
//! has an account (owner) that can be granted exclusive access to specific
//! functions.
//!
//! The `Ownable` trait exposes methods for:
//! - Getting the current owner
//! - Transferring ownership
//! - Renouncing ownership
//!
//! The helper `enforce_owner_auth()` is available to restrict access to only
//! the owner. You can also use the `#[only_owner]` macro (provided elsewhere)
//! to simplify this.
//!
//! ```ignore
//! #[only_owner]
//! fn set_config(e: &Env, new_config: u32) { ... }
//! ```
//!
//! See `examples/ownable/src/contract.rs` for a working example.
//!
//! ## Note
//!
//! The ownership transfer is processed in 2 steps:
//!
//! 1. Initiating the ownership transfer by the current owner
//! 2. Accepting the ownership by the designated owner
//!
//! Not providing a direct ownership transfer is a deliberate design decision to
//! help avoid mistakes by transferring to a wrong address.

#![no_std]

mod ownable;
mod storage;

pub use crate::{
    ownable::{
        emit_ownership_renounced, emit_ownership_transfer, emit_ownership_transfer_completed,
        Ownable, OwnableError, OwnableExt,
    },
    storage::OwnableStorageKey,
};

pub struct Owner;

impl ownable::Ownable for Owner {
    type Impl = Self;

    fn get_owner(e: &soroban_sdk::Env) -> Option<soroban_sdk::Address> {
        storage::get_owner(e)
    }

    fn transfer_ownership(
        e: &soroban_sdk::Env,
        new_owner: &soroban_sdk::Address,
        live_until_ledger: u32,
    ) {
        storage::transfer_ownership(e, new_owner, live_until_ledger);
    }

    fn accept_ownership(e: &soroban_sdk::Env) {
        storage::accept_ownership(e);
    }

    fn renounce_ownership(e: &soroban_sdk::Env) {
        storage::renounce_ownership(e);
    }

    fn set_owner(e: &soroban_sdk::Env, owner: &soroban_sdk::Address) {
        storage::set_owner(e, owner)
    }
}

mod test;
