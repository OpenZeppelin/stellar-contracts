use soroban_sdk::{contracttype, BytesN, Env};

#[contracttype]
pub enum UpgradeableStorageKey {
    SchemaVersion,
}

/// Returns the current schema version stored in instance storage.
///
/// Defaults to `0` if no schema version has been set yet (e.g. contracts
/// deployed before schema versioning was introduced).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn get_schema_version(e: &Env) -> u32 {
    e.storage().instance().get(&UpgradeableStorageKey::SchemaVersion).unwrap_or(0)
}

/// Sets the schema version in instance storage.
///
/// Call this at the end of a `migrate` function after all storage
/// transformations have been applied, to record that migration to this
/// version is complete and prevent re-execution.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `version` - The schema version to record.
///
/// # Security Warning
///
/// **IMPORTANT**: This function lacks authorization checks and should only
/// be used in admin functions that implement their own authorization logic.
pub fn set_schema_version(e: &Env, version: u32) {
    e.storage().instance().set(&UpgradeableStorageKey::SchemaVersion, &version);
}

/// Updates the contract WASM bytecode.
///
/// The contract will only be upgraded after the invocation has successfully
/// completed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `new_wasm_hash` - A 32-byte hash identifying the new WASM blob, uploaded
///   to the ledger.
///
/// # Security Warning
///
/// **IMPORTANT**: This function lacks authorization checks and should only
/// be used in admin functions that implement their own authorization logic.
pub fn upgrade(e: &Env, new_wasm_hash: &BytesN<32>) {
    e.deployer().update_current_contract_wasm(new_wasm_hash.clone());
}
