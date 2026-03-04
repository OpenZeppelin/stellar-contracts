//! Concrete compliance module implementations aligned with the RWA modular
//! compliance hooks.
//!
//! ## Design Intent
//! - Keep each compliance rule isolated in its own file and test scope.
//! - Keep cross-cutting concerns centralized in `common`.
//! - Keep all module state token-scoped so one compliance contract can safely
//!   serve many tokens.
//!
//! ## Architecture Notes
//! See `README.md` in this directory for maintainability/scalability rationale,
//! parity notes, and recommended evolution steps for identity adapters.

pub mod common;
pub mod country_allow;
pub mod country_restrict;
pub mod initial_lockup_period;
pub mod max_balance;
pub mod supply_limit;
#[cfg(test)]
pub mod test_utils;
pub mod time_transfers_limits;
pub mod transfer_restrict;
