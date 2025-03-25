use soroban_sdk::{contractclient, contracterror, Address, BytesN, Env, FromVal, Val};

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

/// High-level trait representing upgradeable contracts that support migrations
/// and rollbacks.
///
/// This trait should be used with the `#[derive(Upgradeable)]` and/or
/// `#[migratable]` macros to automate implementation. It exposes the `migrate`
/// and `rollback` entry points for upgrade lifecycle handling.
pub trait Migratable: MigratableInternal {
    /// Entry point to handle a contract migration. Can only be called during
    /// upgrade.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `migration_data` - Arbitrary data passed to the migration logic.
    fn migrate(e: &Env, migration_data: Self::MigrationData);

    /// Entry point to handle a rollback of a migration.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `rollback_data` - Arbitrary data passed to the rollback logic.
    fn rollback(e: &Env, rollback_data: Self::RollbackData);
}

/// Trait to be implemented for a custom upgrade authorization mechanism.
/// Requires defining custom access control logic for who can upgrade the
/// contract.
pub trait UpgradeableInternal {
    /// Ensures the `operator` is authorized to perform the upgrade.
    ///
    /// This must be implemented by the consuming contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `operator` - The address attempting the upgrade. Can be C-account or
    ///   antoher contract such as timelock or governor.
    fn _upgrade_auth(e: &Env, operator: &Address);
}

/// Trait to be implemented for custom migration and rollback behavior. Provides
/// fine-grained control over data transformations during upgrades.
pub trait MigratableInternal {
    /// Type representing structured data needed during migration.
    type MigrationData: FromVal<Env, Val>;

    /// Type representing structured data needed during rollback.
    type RollbackData: FromVal<Env, Val>;

    /// Applies migration logic using the given data. Must be implemented by the
    /// consuming contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `migration_data` - Migration-specific input data.
    fn _migrate(e: &Env, migration_data: &Self::MigrationData);

    /// Applies rollback logic using the given data. Must be implemented by the
    /// consuming contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `rollback_data` - Rollback-specific input data.
    fn _rollback(e: &Env, rollback_data: &Self::RollbackData);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UpgradeableError {
    /// When migration is attempted but not allowed due to upgrade state.
    MigrationNotAllowed = 200,
    /// When rollback is attempted but not allowed due to upgrade state.
    RollbackNotAllowed = 201,
}
