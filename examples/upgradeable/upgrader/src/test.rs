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
    
    // Initialize the upgrader with an admin
    upgrader_client.initialize(&admin);
    
    // Verify the admin was set correctly
    assert_eq!(upgrader_client.get_admin(), admin);

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

#[test]
fn test_admin_transfer_successful() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let initial_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    let upgrader = env.register(Upgrader, ());
    let upgrader_client = UpgraderClient::new(&env, &upgrader);
    
    // Initialize the upgrader with an admin
    upgrader_client.initialize(&initial_admin);
    
    // Verify initial admin was set correctly
    assert_eq!(upgrader_client.get_admin(), initial_admin);
    
    // Transfer admin rights
    upgrader_client.set_admin(&initial_admin, &new_admin);
    
    // Verify the admin was updated
    assert_eq!(upgrader_client.get_admin(), new_admin);
    
    // New admin should be able to perform admin actions
    let another_admin = Address::generate(&env);
    upgrader_client.set_admin(&new_admin, &another_admin);
    
    // Verify the admin was updated again
    assert_eq!(upgrader_client.get_admin(), another_admin);
}

#[test]
#[should_panic(expected = "not authorized")]
fn test_upgrade_with_upgrader_unauthorized() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let not_admin = Address::generate(&env);
    let contract_id = env.register(contract_v1::WASM, (&admin,));

    let upgrader = env.register(Upgrader, ());
    let upgrader_client = UpgraderClient::new(&env, &upgrader);
    
    // Initialize the upgrader with an admin
    upgrader_client.initialize(&admin);
    
    let new_wasm_hash = install_new_wasm(&env);
    
    // This should panic with "not authorized"
    upgrader_client.upgrade(
        &contract_id,
        &not_admin,
        &new_wasm_hash,
    );
}

#[test]
#[should_panic(expected = "not authorized")]
fn test_admin_transfer_unauthorized() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let initial_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    let upgrader = env.register(Upgrader, ());
    let upgrader_client = UpgraderClient::new(&env, &upgrader);
    
    // Initialize the upgrader with an admin
    upgrader_client.initialize(&initial_admin);
    
    // Verify initial admin was set correctly
    assert_eq!(upgrader_client.get_admin(), initial_admin);
    
    // Transfer admin rights
    upgrader_client.set_admin(&initial_admin, &new_admin);
    
    // Verify the admin was updated
    assert_eq!(upgrader_client.get_admin(), new_admin);
    
    // Try to set admin using the old admin (should panic with "not authorized")
    upgrader_client.set_admin(&initial_admin, &initial_admin);
}
