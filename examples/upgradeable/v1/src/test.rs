#![cfg(test)]

extern crate std;

use contract_v2::Data;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

mod contract_v2 {
    use crate::test::MigrationData;
    use crate::test::RollbackData;

    soroban_sdk::contractimport!(
        file = "../../../target/wasm32-unknown-unknown/release/upgradeable_v2_example.wasm"
    );
}
//const WASM_AFTER_UPGRADE: &[u8] = include_bytes!("testdata/dummy.wasm");

//mod contract_v1 {
//soroban_sdk::contractimport!(
//file = "../../../../target/wasm32-unknown-unknown/release/upgradeable_v1_example.wasm"
//);
//}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

//fn install_old_wasm(e: &Env) -> BytesN<32> {
//e.deployer().upload_contract_wasm(contract_v1::WASM)
//}

type MigrationData = Data;
type RollbackData = Data;

#[test]
fn test_upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(ExampleContract, (&admin,));

    let client_v1 = ExampleContractClient::new(&env, &contract_id);

    let new_wasm_hash = install_new_wasm(&env);

    client_v1.upgrade(&new_wasm_hash);

    let client_v2 = contract_v2::Client::new(&env, &contract_id);
    client_v2.migrate(&Data { num1: 12, num2: 34 });

    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }).is_err());

    client_v2.rollback(&Data { num1: 23, num2: 45 });

    assert!(client_v2.try_rollback(&Data { num1: 23, num2: 45 }).is_err());
    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }).is_err());
}
