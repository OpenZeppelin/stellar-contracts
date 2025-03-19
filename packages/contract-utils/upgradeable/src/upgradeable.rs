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
    /// * `new_wasm_hash` - The identifier of the WASM blob, uploaded to the
    ///   ledger.
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address);
}

// No need to manually implement this trait as it can be derived with
// #[derive(Upgradeable)] and #[migratable]
pub trait Migratable: MigratableInternal {
    fn migrate(e: &Env, migration_data: Self::MigrationData);
    fn rollback(e: &Env, rollback_data: Self::RollbackData);
}

// Trait to be implemented for a concrete upgrade procedure.
pub trait UpgradeableInternal {
    // needs to implement access control
    // `operator` can be C-account (e.g. owner) or antoher contract such as timelock or governor
    fn _upgrade_auth(e: &Env, operator: &Address);
}

// Trait to be implemented for a concrete migrate procedure.
pub trait MigratableInternal {
    type MigrationData: FromVal<Env, Val>;
    type RollbackData: FromVal<Env, Val>;

    // needs to implement access control and migrate logic
    fn _migrate(e: &Env, migration_data: &Self::MigrationData);

    // needs to implement access control and rollback logic
    fn _rollback(e: &Env, rollback_data: &Self::RollbackData);
}

// TODO: EVENT for migration ?

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UpgradeableError {
    MigrationNotAllowed = 200,
    RollbackNotAllowed = 201,
}
