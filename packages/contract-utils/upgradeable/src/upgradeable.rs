use soroban_sdk::{contractclient, contracterror, BytesN, Env, FromVal, Val};

#[contractclient(name = "UpgradeableClient")]
pub trait Upgradeable {
    /// Upgrades the contract by setting a new WASM bytecode. The contract will only be upgraded after the invocation has successfully completed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_wasm_hash` - The identifier of the WASM blob, uploaded to the ledger.
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>);
}

pub trait Migrateable: Migration {
    fn migrate(e: &Env, migration_data: Self::MigrationData);
    fn rollback(e: &Env, rollback_data: Self::RollbackData);
}

pub trait Upgrade {
    fn _upgrade(e: &Env, new_wasm_hash: &BytesN<32>);
}

pub trait Migration {
    type MigrationData: FromVal<Env, Val>;
    type RollbackData: FromVal<Env, Val>;

    fn _migrate(e: &Env, migration_data: &Self::MigrationData);
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
