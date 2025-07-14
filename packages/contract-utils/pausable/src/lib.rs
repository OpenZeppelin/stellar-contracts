//! Pausable Contract Module.
//!
//! This contract module allows implementing a configurable stop mechanism for
//! your contract.
//!
//! By implementing the trait [`Pausable`] for your contract, you can safely
//! integrate the Pausable functionality. The trait [`Pausable`] has the
//! following methods:
//! - [`paused()`]
//! - [`pause()`]
//! - [`unpause()`]
//!
//! The trait ensures all the required methods are implemented for your
//! contract, and nothing is forgotten. Additionally, if you are to implement
//! multiple extensions or utilities for your contract, the code will be better
//! organized.
//!
//! We also provide two macros `when_paused` and `when_not_paused`. These macros
//! act as guards for your functions. For example:
//!
//! ```ignore
//! #[when_not_paused]
//! fn transfer(e: &env, from: Address, to: Address) {
//!     /* this body will execute ONLY when NOT_PAUSED */
//! }
//! ```
//!
//! For a safe pause/unpause implementation, we expose the underlying functions
//! required for the pausing. These functions work with the Soroban environment
//! required for the Smart Contracts `e: &Env`, and take advantage of the
//! storage by storing a flag for the pause mechanism.
//!
//! We expect you to utilize these functions (`storage::*`) for implementing the
//! methods of the `Pausable` trait, along with your custom business logic
//! (authentication, etc.)
//!
//! You can opt-out of [`Pausable`] trait, and use `storage::*` functions
//! directly in your contract if you want more customizability. But we encourage
//! the use of [`Pausable`] trait instead, due to the following reasons:
//! - there is no additional cost
//! - standardization
//! - you cannot mistakenly forget one of the methods
//! - your code will be better organized, especially if you implement multiple
//!   extensions/utils
//!
//! TL;DR
//! to see it all in action, check out the `examples/pausable/src/contract.rs`
//! file.

#![no_std]

mod pausable;
mod storage;

pub use crate::{
    pausable::{emit_paused, emit_unpaused, Pausable, PausableError, PausableExt},
    storage::{pause, paused, unpause, when_not_paused, when_paused, PausableDefault},
};

mod test;
