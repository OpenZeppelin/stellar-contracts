extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol};

use crate::contract::{ExampleContract, ExampleContractClient};

mod contract_v2 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

#[test]
fn test_upgrade_and_migrate() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let migrator = Address::generate(&e);

    // deploy v1 with initial config
    let address = e.register(ExampleContract, (&admin, &100u32));
    let client_v1 = ExampleContractClient::new(&e, &address);

    // verify v1 data is stored correctly
    assert_eq!(client_v1.get_rate(), 100);

    // grant roles and upgrade
    client_v1.grant_role(&manager, &Symbol::new(&e, "manager"), &admin);
    client_v1.grant_role(&migrator, &Symbol::new(&e, "migrator"), &admin);
    let new_wasm_hash = install_new_wasm(&e);
    client_v1.upgrade(&new_wasm_hash, &manager);

    // migrate: reads old Config { rate }, converts to Config { rate, active }
    let client_v2 = contract_v2::Client::new(&e, &address);
    client_v2.migrate(&migrator);

    // verify data was preserved and new field has its default
    assert_eq!(client_v2.get_rate(), 100);
    assert!(client_v2.is_active());

    // ensure migrate can't be invoked again (schema version guard)
    assert!(client_v2.try_migrate(&admin).is_err());
}
