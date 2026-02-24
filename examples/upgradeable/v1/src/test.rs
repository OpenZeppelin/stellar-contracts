extern crate std;

use contract_v2::Data;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol};

use crate::contract::{ExampleContract, ExampleContractClient};

mod contract_v2 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

#[test]
fn test_upgrade() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let migrator = Address::generate(&e);
    // deploy v1
    let address = e.register(ExampleContract, (&admin,));

    let client_v1 = ExampleContractClient::new(&e, &address);
    client_v1.grant_role(&manager, &Symbol::new(&e, "manager"), &admin);
    client_v1.grant_role(&migrator, &Symbol::new(&e, "migrator"), &admin);

    // install the new wasm and upgrade
    let new_wasm_hash = install_new_wasm(&e);
    client_v1.upgrade(&new_wasm_hash, &manager);

    // init the upgraded client and migrate
    let client_v2 = contract_v2::Client::new(&e, &address);
    client_v2.migrate(&Data { num1: 12, num2: 34 }, &migrator);

    // ensure migrate can't be invoked again
    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }, &admin).is_err());
}
