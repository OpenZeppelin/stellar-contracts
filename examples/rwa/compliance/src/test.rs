extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};
use stellar_tokens::rwa::compliance::ComplianceHook;

use crate::contract::{ComplianceContract, ComplianceContractClient};

fn create_client(e: &Env) -> (Address, ComplianceContractClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(ComplianceContract, (&admin,));
    let client = ComplianceContractClient::new(e, &contract_id);
    (admin, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);

    // Initially no modules should be registered
    let modules = client.get_modules_for_hook(&ComplianceHook::CanTransfer);
    assert_eq!(modules.len(), 0);
}

#[test]
fn test_module_management() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);
    let module = Address::generate(&e);

    // Add module
    client.add_module_to(&ComplianceHook::CanTransfer, &module, &admin);

    assert!(client.is_module_registered(&ComplianceHook::CanTransfer, &module));
    let modules = client.get_modules_for_hook(&ComplianceHook::CanTransfer);
    assert_eq!(modules.len(), 1);
    assert_eq!(modules.get(0).unwrap(), module);

    // Remove module
    client.remove_module_from(&ComplianceHook::CanTransfer, &module, &admin);
    assert!(!client.is_module_registered(&ComplianceHook::CanTransfer, &module));
    assert_eq!(client.get_modules_for_hook(&ComplianceHook::CanTransfer).len(), 0);
}

#[test]
fn test_compliance_hooks() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    // Without modules, all operations should be allowed
    assert!(client.can_transfer(&from, &to, &amount));
    assert!(client.can_create(&to, &amount));

    // Test hook calls (these won't fail even without modules)
    client.transferred(&from, &to, &amount);
    client.created(&to, &amount);
    client.destroyed(&from, &amount);
}
