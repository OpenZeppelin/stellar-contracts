extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env};
use stellar_tokens::rwa::compliance::ComplianceHook;

use crate::contract::{ComplianceContract, ComplianceContractClient};

fn create_client<'a>(e: &Env, admin: &Address, manager: &Address) -> ComplianceContractClient<'a> {
    let address = e.register(ComplianceContract, (admin, manager));
    ComplianceContractClient::new(e, &address)
}

// ################## MOCK COMPLIANCE MODULE ##################

/// A mock compliance module that allows all even amounts and rejects odd ones.
#[contract]
struct MockModule;

#[contractimpl]
impl MockModule {
    pub fn on_transfer(_e: Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    pub fn on_created(_e: Env, _to: Address, _amount: i128, _token: Address) {}

    pub fn on_destroyed(_e: Env, _from: Address, _amount: i128, _token: Address) {}

    pub fn can_transfer(
        _e: Env,
        _from: Address,
        _to: Address,
        amount: i128,
        _token: Address,
    ) -> bool {
        amount % 2 == 0
    }

    pub fn can_create(_e: Env, _to: Address, amount: i128, _token: Address) -> bool {
        amount % 2 == 0
    }
}

// ################## MODULES ##################

#[test]
fn add_and_get_module_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let module = Address::generate(&e);

    assert!(!client.is_module_registered(&ComplianceHook::Transferred, &module));
    assert!(client.get_modules_for_hook(&ComplianceHook::Transferred).is_empty());

    client.add_module_to(&ComplianceHook::Transferred, &module, &manager);

    assert!(client.is_module_registered(&ComplianceHook::Transferred, &module));
    assert_eq!(client.get_modules_for_hook(&ComplianceHook::Transferred).len(), 1);
}

#[test]
fn remove_module_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let module = Address::generate(&e);

    client.add_module_to(&ComplianceHook::CanTransfer, &module, &manager);
    assert!(client.is_module_registered(&ComplianceHook::CanTransfer, &module));

    client.remove_module_from(&ComplianceHook::CanTransfer, &module, &manager);
    assert!(!client.is_module_registered(&ComplianceHook::CanTransfer, &module));
    assert!(client.get_modules_for_hook(&ComplianceHook::CanTransfer).is_empty());
}

#[test]
#[should_panic(expected = "Error(Contract, #360)")]
fn add_module_already_registered_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let module = Address::generate(&e);

    client.add_module_to(&ComplianceHook::Created, &module, &manager);
    client.add_module_to(&ComplianceHook::Created, &module, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #361)")]
fn remove_unregistered_module_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let module = Address::generate(&e);

    client.remove_module_from(&ComplianceHook::Destroyed, &module, &manager);
}

// ################## TOKEN BINDING ##################

#[test]
fn bind_and_unbind_token_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);

    assert!(client.linked_tokens().is_empty());

    client.bind_token(&token, &manager);
    assert_eq!(client.linked_tokens().len(), 1);

    client.unbind_token(&token, &manager);
    assert!(client.linked_tokens().is_empty());
}

#[test]
fn bind_tokens_batch_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let tokens: soroban_sdk::Vec<Address> =
        soroban_sdk::vec![&e, Address::generate(&e), Address::generate(&e), Address::generate(&e),];

    client.bind_tokens(&tokens, &manager);
    assert_eq!(client.linked_tokens().len(), 3);
}

// ################## HOOKS ##################

#[test]
fn can_transfer_no_modules_returns_true() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);

    client.bind_token(&token, &manager);

    let from = Address::generate(&e);
    let to = Address::generate(&e);

    assert!(client.can_transfer(&from, &to, &500, &token));
}

#[test]
fn can_create_no_modules_returns_true() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);

    client.bind_token(&token, &manager);

    let to = Address::generate(&e);

    assert!(client.can_create(&to, &100, &token));
}

#[test]
fn can_transfer_module_filters_odd_amounts() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);
    let module = e.register(MockModule, ());

    client.bind_token(&token, &manager);
    client.add_module_to(&ComplianceHook::CanTransfer, &module, &manager);

    let from = Address::generate(&e);
    let to = Address::generate(&e);

    assert!(client.can_transfer(&from, &to, &1000, &token));
    assert!(!client.can_transfer(&from, &to, &1001, &token));
}

#[test]
fn can_create_module_filters_odd_amounts() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);
    let module = e.register(MockModule, ());

    client.bind_token(&token, &manager);
    client.add_module_to(&ComplianceHook::CanCreate, &module, &manager);

    let to = Address::generate(&e);

    assert!(client.can_create(&to, &200, &token));
    assert!(!client.can_create(&to, &201, &token));
}

#[test]
fn transferred_hook_executes_modules() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);
    let module = e.register(MockModule, ());

    client.bind_token(&token, &manager);
    client.add_module_to(&ComplianceHook::Transferred, &module, &manager);

    let from = Address::generate(&e);
    let to = Address::generate(&e);

    client.transferred(&from, &to, &500, &token);
}

#[test]
fn created_hook_executes_modules() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);
    let module = e.register(MockModule, ());

    client.bind_token(&token, &manager);
    client.add_module_to(&ComplianceHook::Created, &module, &manager);

    let to = Address::generate(&e);

    client.created(&to, &500, &token);
}

#[test]
fn destroyed_hook_executes_modules() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let token = Address::generate(&e);
    let module = e.register(MockModule, ());

    client.bind_token(&token, &manager);
    client.add_module_to(&ComplianceHook::Destroyed, &module, &manager);

    let from = Address::generate(&e);

    client.destroyed(&from, &500, &token);
}

#[test]
#[should_panic(expected = "Error(Contract, #363)")]
fn transferred_unbound_token_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let unbound_token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    client.transferred(&from, &to, &100, &unbound_token);
}
