use soroban_sdk::{contract, contracttype, symbol_short, Address, BytesN, Env, Symbol};

use stellar_upgradeable::{Migration, Upgrade};
use stellar_upgradeable_macros::Upgradeable;

pub const DATA_KEY: Symbol = symbol_short!("DATA_KEY");
pub const OWNER: Symbol = symbol_short!("OWNER");

#[contracttype]
pub struct Data {
    pub num1: u32,
    pub num2: u32,
}

#[derive(Upgradeable)]
#[migrateable]
#[contract]
pub struct ExampleContract;

impl Upgrade for ExampleContract {
    fn _upgrade(e: &Env, new_wasm_hash: &BytesN<32>) {
        let owner: Address = e.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();

        e.deployer().update_current_contract_wasm(new_wasm_hash.clone());
    }
}

impl Migration for ExampleContract {
    type MigrationData = Data;
    type RollbackData = Data;

    fn _migrate(e: &Env, data: &Self::MigrationData) {
        let owner: Address = e.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();
        e.storage().instance().set(&DATA_KEY, data);
    }

    fn _rollback(e: &Env, _data: &Self::RollbackData) {
        let owner: Address = e.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();
        e.storage().instance().remove(&DATA_KEY);
    }
}
