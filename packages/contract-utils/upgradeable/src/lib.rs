//! # Lightweight upgradeability framework
//!
//! This module defines a minimal system for managing contract upgrades, with
//! optional support for handling migrations in a structured and safe manner.
//! The framework enforces correct sequencing of operations, e.g. migration can
//! only be invoked after an upgrade.
//!
//! It is recommended to use this module via the `#[derive(Upgradeable)]` macro,
//! or via the `#[derive(UpgradeableMigratable)]` when custom migration logic is
//! additionally needed.
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
//! Example for upgrade only:
//! ```rust,ignore
//! #[derive(Upgradeable)]
//! #[contract]
//! pub struct ExampleContract;
//!
//! impl UpgradeableInternal for ExampleContract {
//!     fn _require_auth(e: &Env, operator: &Address) {
//!         operator.require_auth();
//!         let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
//!         if *operator != owner {
//!             panic_with_error!(e, ExampleContractError::Unauthorized)
//!         }
//!     }
//! }
//! ```
//!
//! # Example for upgrade and migration:
//! ```ignore,rust
//! #[contracttype]
//! pub struct Data {
//!     pub num1: u32,
//!     pub num2: u32,
//! }
//!
//! #[derive(UpgradeableMigratable)]
//! #[contract]
//! pub struct ExampleContract;
//!
//! impl UpgradeableMigratableInternal for ExampleContract {
//!     type MigrationData = Data;
//!
//!     fn _require_auth(e: &Env, operator: &Address) {
//!         operator.require_auth();
//!         let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
//!         if *operator != owner {
//!             panic_with_error!(e, ExampleContractError::Unauthorized)
//!         }
//!     }
//!
//!     fn _migrate(e: &Env, data: &Self::MigrationData) {
//!         e.storage().instance().set(&DATA_KEY, data);
//!     }
//! }
//! ```
//! Check in the "/examples/upgradeable/" directory for the full example, where
//! you can also find a helper `Upgrader` contract that performs upgrade+migrate
//! in a single transaction.

#![no_std]

mod storage;
mod test;
mod upgradeable;

pub use crate::{
    storage::{
        can_complete_migration, complete_migration, enable_migration, ensure_can_complete_migration,
    },
    upgradeable::{
        Upgradeable, UpgradeableClient, UpgradeableInternal, UpgradeableMigratable,
        UpgradeableMigratableInternal,
    },
};
