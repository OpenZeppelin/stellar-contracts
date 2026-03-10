//! Compliance module library — traits with default implementations,
//! events, errors, and storage functions for T-REX compliance modules.
//!
//! Each sub-module defines a `#[contracttrait]` trait whose default method
//! bodies contain the full business logic. Concrete contracts compose
//! these traits via `#[contractimpl(contracttrait)]` in the examples.

pub mod common;

pub use super::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};
