use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol, Val};
use stellar_upgradeable::UpgradeableClient;

pub const MIGRATE: Symbol = symbol_short!("migrate");

#[contract]
pub struct Upgrader;

#[contractimpl]
impl Upgrader {
    pub fn upgrade(
        env: Env,
        contract_address: Address,
        operator: Address,
        new_wasm_hash: BytesN<32>,
        migration_data: soroban_sdk::Vec<Val>,
    ) {
        let contract_client = UpgradeableClient::new(&env, &contract_address);

        contract_client.upgrade(&new_wasm_hash, &operator);
        // The types of the arguments to the migrate function are unknown to this contract, so we need to call it with invoke_contract.
        // The migrate function's return value can be safely cast to () no matter what it really is,
        // because it will panic on failure anyway
        env.invoke_contract::<()>(&contract_address, &MIGRATE, migration_data);
    }
}
