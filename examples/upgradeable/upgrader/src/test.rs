extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol, TryIntoVal};

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
    let manager = Address::generate(&e);
    let migrator = Address::generate(&e);
    let contract_id = e.register(contract_v1::WASM, (&admin, &100u32));

    let client_v1 = contract_v1::Client::new(&e, &contract_id);
    client_v1.grant_role(&manager, &Symbol::new(&e, "manager"), &admin);
    client_v1.grant_role(&migrator, &Symbol::new(&e, "migrator"), &admin);

    let upgrader = e.register(Upgrader, (&admin,));
    let upgrader_client = UpgraderClient::new(&e, &upgrader);

    let new_wasm_hash = install_new_wasm(&e);

    upgrader_client.upgrade_and_migrate(
        &contract_id,
        &manager,
        &new_wasm_hash,
        &soroban_sdk::vec![&e, migrator.try_into_val(&e).unwrap()],
    );

    let client_v2 = contract_v2::Client::new(&e, &contract_id);

    // verify migration happened: data preserved and new field set
    assert_eq!(client_v2.get_rate(), 100);
    assert!(client_v2.is_active());

    // ensure migrate can't be invoked again
    assert!(client_v2.try_migrate(&admin).is_err());
}
