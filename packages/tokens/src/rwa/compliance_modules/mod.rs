//! Compliance module library — traits with default implementations,
//! events, errors, and storage functions for T-REX compliance modules.
//!
//! Each sub-module defines a `#[contracttrait]` trait whose default method
//! bodies contain the full business logic. Concrete contracts compose
//! these traits via `#[contractimpl(contracttrait)]` in the examples.

pub mod common;
pub mod country_allow;
pub mod country_restrict;
pub mod initial_lockup_period;
pub mod max_balance;
pub mod supply_limit;
pub mod time_transfers_limits;
pub mod transfer_restrict;

pub use super::{ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};
