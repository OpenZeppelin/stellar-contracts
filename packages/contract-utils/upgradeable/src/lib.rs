//! # Lightweight upgradeability framework
//!
//! This module defines a minimal system for managing contract upgrades, with
//! optional support for handling migrations and rollbacks in a structured and
//! safe manner.
//!
//! The framework enforces correct sequencing of operations (e.g. migration can
//! only be invoked after an upgrade or rollback is only allowed after a
//! migration), ensuring safe and predictable transitions.
//!
//! It is recommended to use this module via the `#[derive(Upgradeable)]` macro,
//! or via the combination `#[derive(Upgradeable, Migratable)]` when custom
//! migration or rollback logic is additionally needed.
//!
//! **Important**: While the framework structures the upgrade flow, it does NOT
//! perform deeper checks and verifications such as:
//!
//! - Ensuring that the new contract does not include a constructor, as it will
//!   not be invoked.
//! - Verifying that the new contract includes an upgradability mechanism,
//!   preventing an unintended loss of upgradability.
//! - Checking for storage consistency, ensuring that the new contract does not
//!   inadvertently introduce storage mismatches.
//!
//! # Example
//! ```ignore,rust
//! #[contracttype]
//! pub struct Data {
//!     pub num1: u32,
//!     pub num2: u32,
//! }
//!
//! #[derive(Upgradeable, Migratable)]
//! #[contract]
//! pub struct ExampleContract;
//!
//! impl UpgradeableInternal for ExampleContract {
//!     fn _upgrade_auth(e: &Env, operator: &Address) {
//!         operator.require_auth();
//!         let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
//!         if *operator != owner {
//!             panic_with_error!(e, ExampleContractError::Unauthorized)
//!         }
//!     }
//! }
//!
//! impl MigratableInternal for ExampleContract {
//!     type MigrationData = Data;
//!     type RollbackData = ();
//!
//!     fn _migrate(e: &Env, data: &Self::MigrationData) {
//!         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
//!         e.storage().instance().set(&DATA_KEY, data);
//!     }
//!
//!     fn _rollback(e: &Env, _data: &Self::RollbackData) {
//!         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
//!         e.storage().instance().remove(&DATA_KEY);
//!     }
//! }
//! ```
//! Check in the "/examples/upgradeable/" directory for the full example, where
//! can also be found a helper `Upgrader` contract that performs upgrade+migrate
//! or rollback+downgrade in a single transaction.

#![no_std]

mod storage;
mod test;
mod upgradeable;

pub use crate::{
    storage::{
        can_migrate, can_rollback, complete_migration, complete_rollback, ensure_can_migrate,
        ensure_can_rollback, start_migration,
    },
    upgradeable::{
        Migratable, MigratableInternal, Upgradeable, UpgradeableClient, UpgradeableInternal,
    },
};
