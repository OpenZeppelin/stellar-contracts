//! # Lightweight upgradeability framework
//!
//! This module defines a minimal system for managing contract upgrades, with
//! optional support for handling migrations in a structured and safe manner.
//! The framework enforces correct sequencing of operations, e.g. migration can
//! only be invoked after an upgrade.
//!
//! If a rollback is required, the contract can be upgraded to a newer version
//! where the rollback-specific logic is defined and performed as a migration.
//!
//! **IMPORTANT**: While the framework structures the upgrade flow, it does NOT
//! perform deeper checks and verifications such as:
//!
//! - Ensuring that the new contract does not include a constructor, as it will
//!   not be invoked.
//! - Verifying that the new contract includes an upgradability mechanism,
//!   preventing an unintended loss of further upgradability capacity.
//! - Checking for storage consistency, ensuring that the new contract does not
//!   inadvertently introduce storage mismatches.
//!
//!
//! ## Simple Upgrade (no migration)
//!
//! Implement the [`Upgradeable`] trait directly and call [`upgrade()`] inside:
//!
//! ```rust,ignore
//! use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};
//! use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
//! use stellar_macros::only_role;
//!
//! #[contract]
//! pub struct ExampleContract;
//!
//! #[contractimpl]
//! impl Upgradeable for ExampleContract {
//!     #[only_role(operator, "admin")]
//!     fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
//!         upgradeable::upgrade(e, &new_wasm_hash);
//!     }
//! }
//! ```
//!
//! ## Upgrade with Migration
//!
//! Implement [`Upgradeable`] as above, and add a `migrate` function to your
//! contract that calls [`run_migration()`] which will prevent you from calling
//! the `migrate` function twice.
//!
//! ```rust,ignore
//! use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env};
//! use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
//! use stellar_macros::only_role;
//!
//! #[contracttype]
//! pub struct Data {
//!     pub num1: u32,
//!     pub num2: u32,
//! }
//!
//! #[contract]
//! pub struct ExampleContract;
//!
//! #[contractimpl]
//! impl Upgradeable for ExampleContract {
//!     #[only_role(operator, "admin")]
//!     fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
//!         upgradeable::upgrade(e, &new_wasm_hash);
//!     }
//! }
//!
//! #[contractimpl]
//! impl ExampleContract {
//!     #[only_role(operator, "manager")]
//!     pub fn migrate(e: &Env, migration_data: Data, operator: Address) {
//!         upgradeable::run_migration(e, || {
//!             e.storage().instance().set(&symbol_short!("DATA_KEY"), &migration_data);
//!         });
//!     }
//! }
//! ```
//!
//! ## Migration Guide from `#[derive(Upgradeable)]`
//!
//! **Before:**
//! ```rust,ignore
//! use stellar_contract_utils::upgradeable::UpgradeableInternal;
//! use stellar_macros::Upgradeable;
//!
//! #[derive(Upgradeable)]
//! #[contract]
//! pub struct ExampleContract;
//!
//! impl UpgradeableInternal for ExampleContract {
//!     fn _require_auth(e: &Env, operator: &Address) {
//!         operator.require_auth();
//!         // access control checks ...
//!     }
//! }
//! ```
//!
//! **After:**
//! ```rust,ignore
//! use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
//!
//! #[contract]
//! pub struct ExampleContract;
//!
//! #[contractimpl]
//! impl Upgradeable for ExampleContract {
//!     fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
//!         operator.require_auth();
//!         // access control checks ...
//!         upgradeable::upgrade(e, &new_wasm_hash);
//!     }
//! }
//! ```
//!
//! Check in the `/examples/upgradeable/` directory for the full example, where
//! you can also find a helper `Upgrader` contract that performs upgrade+migrate
//! in a single transaction.

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractclient, contracterror, Address, BytesN, Env};

pub use crate::upgradeable::storage::{
    can_complete_migration, complete_migration, enable_migration, ensure_can_complete_migration,
    run_migration, upgrade,
};

/// A trait exposing an entry point for contract upgrades.
///
/// All access control and authorization checks are the implementor's
/// responsibility.
///
/// # Example
///
/// ```rust,ignore
/// #[contractimpl]
/// impl Upgradeable for MyContract {
///     fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
///         operator.require_auth();
///         // ... access control ...
///         upgradeable::upgrade(e, &new_wasm_hash);
///     }
/// }
/// ```
#[contractclient(name = "UpgradeableClient")]
pub trait Upgradeable {
    /// Upgrades the contract by setting a new WASM bytecode. The
    /// contract will only be upgraded after the invocation has
    /// successfully completed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_wasm_hash` - A 32-byte hash identifying the new WASM blob,
    ///   uploaded to the ledger.
    /// * `operator` - The authorized address performing the upgrade.
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UpgradeableError {
    /// When migration is attempted but not allowed due to upgrade state.
    MigrationNotAllowed = 1100,
}
