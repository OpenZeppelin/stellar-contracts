#![cfg(test)]

extern crate std;

use contract_v2::Data;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, TryIntoVal};

use crate::contract::{Upgrader, UpgraderClient};

mod contract_v1 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v1_example.wasm");
}

mod contract_v2 {
    use crate::test::{MigrationData, RollbackData};

    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

fn install_old_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v1::WASM)
}

type MigrationData = Data;
type RollbackData = ();

#[test]
fn test_upgrade_with_upgrader() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let contract_id = env.register(contract_v1::WASM, (&admin,));

    let upgrader = env.register(Upgrader, ());
    let upgrader_client = UpgraderClient::new(&env, &upgrader);

    let new_wasm_hash = install_new_wasm(&env);
    let data = Data { num1: 12, num2: 34 };

    upgrader_client.upgrade_and_migrate(
        &contract_id,
        &admin,
        &new_wasm_hash,
        &soroban_sdk::vec![&env, data.try_into_val(&env).unwrap()],
    );

    let old_wasm_hash = install_old_wasm(&env);
    let client_v2 = contract_v2::Client::new(&env, &contract_id);

    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }).is_err());

    upgrader_client.rollback_and_upgrade(
        &contract_id,
        &admin,
        &old_wasm_hash,
        &soroban_sdk::vec![&env, ().into()],
    );

    assert!(client_v2.try_rollback(&()).is_err());
    assert!(client_v2.try_migrate(&data).is_err());
}
