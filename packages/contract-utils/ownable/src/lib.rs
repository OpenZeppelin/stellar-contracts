//! Ownable Contract Module.
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
//! The helper `ensure_is_owner()` is available to restrict access to only the
//! owner. You can also use the `#[only_owner]` macro (provided elsewhere) to
//! simplify this.
//!
//! ```ignore
//! #[only_owner]
//! fn set_config(e: &Env, new_config: u32) { ... }
//! ```
//!
//! See `examples/ownable/src/contract.rs` for a working example.

#![no_std]

mod ownable;
mod storage;

pub use crate::{
    ownable::{
        emit_ownership_renounced, emit_ownership_transfer, emit_ownership_transfer_completed,
        Ownable, OwnableError,
    },
    storage::{
        accept_ownership, ensure_is_owner, get_owner, renounce_ownership, set_owner,
        transfer_ownership, OwnableStorageKey,
    },
};

mod test;
