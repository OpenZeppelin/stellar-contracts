extern crate std;

use contract_v2::Data;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, TryIntoVal};

use crate::contract::{Upgrader, UpgraderClient};

mod contract_v1 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v1_example.wasm");
}

mod contract_v2 {
    use crate::test::MigrationData;

    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

type MigrationData = Data;

#[test]
fn test_upgrade_with_upgrader() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let contract_id = e.register(contract_v1::WASM, (&admin,));

    let upgrader = e.register(Upgrader, (&admin,));
    let upgrader_client = UpgraderClient::new(&e, &upgrader);

    let new_wasm_hash = install_new_wasm(&e);
    let data = Data { num1: 12, num2: 34 };

    upgrader_client.upgrade_and_migrate(
        &contract_id,
        &admin,
        &new_wasm_hash,
        &soroban_sdk::vec![&e, data.try_into_val(&e).unwrap(), admin.try_into_val(&e).unwrap()],
    );

    let client_v2 = contract_v2::Client::new(&e, &contract_id);

    assert!(client_v2.try_migrate(&Data { num1: 12, num2: 34 }, &admin).is_err());
}
