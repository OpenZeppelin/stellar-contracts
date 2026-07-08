//! # Combined Contract Types for Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::fungible::FungibleToken`] implementation, and that single slot
//! decides all overridable behavior. When the behaviors of two extensions
//! have to apply at the same time, a dedicated contract type is needed that
//! combines them. This module hosts those combined contract types:
//!
//! - [`TotalSupplyAllowList`]: the [`crate::fungible::allowlist::AllowList`]
//!   transfer policy with total supply tracking.
//! - [`TotalSupplyBlockList`]: the [`crate::fungible::blocklist::BlockList`]
//!   transfer policy with total supply tracking.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     type ContractType = TotalSupplyAllowList;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleTotalSupply for MyToken {}
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleAllowList for MyToken {
//!     // ...
//! }
//! ```

pub mod storage;

#[cfg(test)]
mod test;

pub use storage::{TotalSupplyAllowList, TotalSupplyBlockList};
