use soroban_sdk::{BytesN, Env};

/// Updates the contract WASM bytecode.
///
/// Call this inside an [`Upgradeable::upgrade`] implementation after all
/// authorization checks have passed. The contract will only be upgraded
/// after the invocation has successfully completed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `new_wasm_hash` - A 32-byte hash identifying the new WASM blob, uploaded
///   to the ledger.
pub fn upgrade(e: &Env, new_wasm_hash: &BytesN<32>) {
    e.deployer().update_current_contract_wasm(new_wasm_hash.clone());
}
