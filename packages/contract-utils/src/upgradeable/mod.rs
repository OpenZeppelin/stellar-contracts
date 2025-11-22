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


mod storage;

#[cfg(test)]
mod test;

#[cfg(feature = "certora")]
pub mod specs;

use soroban_sdk::{contractclient, contracterror, Address, BytesN, Env, FromVal, Val};

pub use crate::upgradeable::storage::{
    can_complete_migration, complete_migration, enable_migration, ensure_can_complete_migration,
};

/// High-level trait for contract upgrades.
///
/// This trait defines the external entry point and can be used in two ways:
///
/// 1. Standalone – Implement this trait directly when full control over access
///    control and upgrade logic is required. In this case, the implementor is
///    responsible for ensuring:
///    - Proper authorization of the `operator`
///    - Versioning management
///
/// 2. Framework-assisted usage – When using the lightweight upgrade framework
///    provided in this module, you should NOT manually implement this trait.
///    Instead:
///    - Derive it using `#[derive(Upgradeable)]`
///    - Provide access control by implementing [`UpgradeableInternal`] with
///      your custom logic
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

/// Trait to be implemented for a custom upgrade authorization mechanism.
/// Requires defining access control logic for who can upgrade the contract.
pub trait UpgradeableInternal {
    /// Ensures the `operator` has signed and is authorized to perform the
    /// upgrade.
    ///
    /// This must be implemented by the consuming contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `operator` - The address attempting the upgrade. Can be G-account, or
    ///   another contract (C-account) such as timelock or governor.
    fn _require_auth(e: &Env, operator: &Address);
}

/// High-level trait for a combination of upgrade and migration logic in
/// upgradeable contracts.
///
/// This trait defines the external entry points for applying both, an upgrade
/// and a migration. It is recommended to be used only as part of the
/// lightweight upgrade framework provided in this module.
///
/// When using the framework, this trait is automatically derived with
/// `#[derive(UpgradeableMigratable)]`, and should NOT be manually implemented.
/// Instead, the contract must define access control via `_require_auth` and
/// provide its custom migration logic by implementing
/// `UpgradeableMigratableInternal`.
pub trait UpgradeableMigratable: UpgradeableMigratableInternal {
    /// Upgrades the contract by setting a new WASM bytecode. The
    /// contract will only be upgraded after the invocation has
    /// successfully completed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_wasm_hash` - A 32-byte hash identifying the new WASM blob,
    ///   uploaded to the ledger.
    /// * `operator` - The authorized address performing the upgrade and the
    ///   migration.
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address);

    /// Entry point to handle a contract migration.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `migration_data` - Arbitrary data passed to the migration logic.
    /// * `operator` - The authorized address performing the upgrade and the
    ///   migration.
    fn migrate(e: &Env, migration_data: Self::MigrationData, operator: Address);
}

/// Trait to be implemented for custom migration. Requires defining access
/// control and custom business logic for a migration after an upgrade.
pub trait UpgradeableMigratableInternal {
    /// Type representing structured data needed during migration.
    type MigrationData: FromVal<Env, Val>;

    /// Applies migration logic using the given data.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `migration_data` - Migration-specific input data.
    fn _migrate(e: &Env, migration_data: &Self::MigrationData);

    /// Ensures the `operator` has signed and is authorized to perform the
    /// upgrade and the migration.
    ///
    /// This must be implemented by the consuming contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `operator` - The address attempting the upgrade and the migration. Can
    ///   be a G-account, or another contract (C-account) such as timelock or
    ///   governor.
    fn _require_auth(e: &Env, operator: &Address);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UpgradeableError {
    /// When migration is attempted but not allowed due to upgrade state.
    MigrationNotAllowed = 1100,
}
