use soroban_sdk::{contractclient, contracterror, Address, BytesN, Env, FromVal, Val};

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
    /// Ensures the `operator` is authorized to perform the upgrade.
    ///
    /// This must be implemented by the consuming contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `operator` - The address attempting the upgrade. Can be C-account or
    ///   another contract such as timelock or governor.
    fn _upgrade_auth(e: &Env, operator: &Address);
}

/// High-level trait for migration and rollback logic in upgradeable contracts.
///
/// This trait defines the external entry points for applying a migration or
/// performing a rollback. It is recommended to be used only as part of the
/// lightweight upgrade framework provided in this module.
///
/// When using the framework, this trait is automatically derived with
/// `#[derive(Migratable)]`, and should NOT be manually implemented. Instead,
/// the contract must provide its custom migration and rollback logic by
/// implementing `MigratableInternal`.
pub trait Migratable: MigratableInternal {
    /// Entry point to handle a contract migration.
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

/// Trait to be implemented for custom migration and rollback behavior. Requires
/// defining access control and custom business logic for a migration after an
/// upgrade, as well as the applicable rollback logic.
pub trait MigratableInternal {
    /// Type representing structured data needed during migration.
    type MigrationData: FromVal<Env, Val>;

    /// Type representing structured data needed during rollback.
    type RollbackData: FromVal<Env, Val>;

    /// Applies migration logic using the given data.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `migration_data` - Migration-specific input data.
    fn _migrate(e: &Env, migration_data: &Self::MigrationData);

    /// Applies rollback logic using the given data.
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
    MigrationNotAllowed = 110,
    /// When rollback is attempted but not allowed due to upgrade state.
    RollbackNotAllowed = 111,
}
