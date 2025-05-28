/// Helper contract to perform upgrade+migrate in a single transaction.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol, Val};
use stellar_ownable::{self as ownable};
use stellar_ownable_macro::only_owner;
use stellar_upgradeable::UpgradeableClient;

pub const MIGRATE: Symbol = symbol_short!("migrate");

#[contract]
pub struct Upgrader;

#[contractimpl]
impl Upgrader {
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }

    #[only_owner]
    pub fn upgrade(e: &Env, contract_address: Address, operator: Address, wasm_hash: BytesN<32>) {
        let contract_client = UpgradeableClient::new(e, &contract_address);

        contract_client.upgrade(&wasm_hash, &operator);
    }

    #[only_owner]
    pub fn upgrade_and_migrate(
        e: &Env,
        contract_address: Address,
        operator: Address,
        wasm_hash: BytesN<32>,
        migration_data: soroban_sdk::Vec<Val>,
    ) {
        let contract_client = UpgradeableClient::new(e, &contract_address);

        contract_client.upgrade(&wasm_hash, &operator);
        // The types of the arguments to the migrate function are unknown to this
        // contract, so we need to call it with invoke_contract.
        e.invoke_contract::<()>(&contract_address, &MIGRATE, migration_data);
    }
}
