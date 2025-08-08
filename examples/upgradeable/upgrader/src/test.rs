#![cfg(test)]

extern crate std;

use contract_v2::Data;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::contract::{Upgrader, UpgraderClient};

mod contract_v1 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v1_example.wasm");
}

mod contract_v2 {

    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

#[test]
fn test_upgrade_with_upgrader() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let contract_id = e.register(contract_v1::WASM, (&admin,));

    let upgrader = e.register(Upgrader, (&admin,));
    let upgrader_client = UpgraderClient::new(&e, &upgrader);

    let new_wasm_hash = install_new_wasm(&e);

    upgrader_client.upgrade(&contract_id, &new_wasm_hash);

    let data = Data { num1: 12, num2: 34 };
    let client_v2 = contract_v2::Client::new(&e, &contract_id);
    client_v2.set_data(&data);

    // ensure migrate can't be invoked again
    assert_eq!(data, client_v2.get_data());
}
