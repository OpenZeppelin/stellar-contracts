/// Helper contract to perform upgrade+migrate or rollback+downgrade in a single
/// transaction.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol, Val};
use stellar_upgradeable::UpgradeableClient;

pub const MIGRATE: Symbol = symbol_short!("migrate");
pub const ROLLBACK: Symbol = symbol_short!("rollback");

#[contract]
pub struct Upgrader;

#[contractimpl]
impl Upgrader {
    pub fn upgrade(env: Env, contract_address: Address, operator: Address, wasm_hash: BytesN<32>) {
        let contract_client = UpgradeableClient::new(&env, &contract_address);

        contract_client.upgrade(&wasm_hash, &operator);
    }

    pub fn upgrade_and_migrate(
        env: Env,
        contract_address: Address,
        operator: Address,
        wasm_hash: BytesN<32>,
        migration_data: soroban_sdk::Vec<Val>,
    ) {
        let contract_client = UpgradeableClient::new(&env, &contract_address);

        contract_client.upgrade(&wasm_hash, &operator);
        // The types of the arguments to the migrate function are unknown to this
        // contract, so we need to call it with invoke_contract.
        env.invoke_contract::<()>(&contract_address, &MIGRATE, migration_data);
    }

    pub fn rollback_and_upgrade(
        env: Env,
        contract_address: Address,
        operator: Address,
        wasm_hash: BytesN<32>,
        rollback_data: soroban_sdk::Vec<Val>,
    ) {
        let contract_client = UpgradeableClient::new(&env, &contract_address);

        // The types of the arguments to the rollback function are unknown to this
        // contract, so we need to call it with invoke_contract.
        env.invoke_contract::<()>(&contract_address, &ROLLBACK, rollback_data);
        contract_client.upgrade(&wasm_hash, &operator);
    }
}
