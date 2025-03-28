#![cfg(test)]

extern crate std;

use contract_v2::Data;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

mod contract_v2 {
    use crate::test::{MigrationData, RollbackData};

    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

type MigrationData = Data;
type RollbackData = ();

#[test]
fn test_upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    // deploy v1
    let address = env.register(ExampleContract, (&admin,));

    let client_v1 = ExampleContractClient::new(&env, &address);

    // install the new wasm and upgrade
    let new_wasm_hash = install_new_wasm(&env);
    client_v1.upgrade(&new_wasm_hash, &admin);

    // init the upgraded client and migrate
    let client_v2 = contract_v2::Client::new(&env, &address);
    client_v2.migrate(&Data { num1: 12, num2: 34 });

    // ensure migrate can't be invoked again
    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }).is_err());

    // rollback
    client_v2.rollback(&());

    // ensure migrate and rollback can't be invoked before another upgrade
    assert!(client_v2.try_rollback(&()).is_err());
    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }).is_err());
}
