extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{TransferLimitModule, TransferLimitModuleClient};

fn create_client(e: &Env) -> (Address, TransferLimitModuleClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(TransferLimitModule, ());
    let client = TransferLimitModuleClient::new(e, &contract_id);
    (admin, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);

    // Test module name
    assert_eq!(client.name(), String::from_str(&e, "Transfer Limit Module"));
}

#[test]
fn test_transfer_limits() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    // Test transfer within limit
    let small_amount = 500_000_000i128; // 500M tokens
    assert!(client.can_transfer(&from, &to, &small_amount));

    // Test transfer at limit
    let max_amount = 1_000_000_000i128; // 1B tokens
    assert!(client.can_transfer(&from, &to, &max_amount));

    // Test transfer exceeding limit
    let large_amount = 2_000_000_000i128; // 2B tokens
    assert!(!client.can_transfer(&from, &to, &large_amount));
}

#[test]
fn test_creation_limits() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let to = Address::generate(&e);

    // Test mint within limit
    let small_amount = 5_000_000_000i128; // 5B tokens
    assert!(client.can_create(&to, &small_amount));

    // Test mint at limit
    let max_amount = 10_000_000_000i128; // 10B tokens
    assert!(client.can_create(&to, &max_amount));

    // Test mint exceeding limit
    let large_amount = 15_000_000_000i128; // 15B tokens
    assert!(!client.can_create(&to, &large_amount));
}

#[test]
fn test_hook_methods() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    // These methods should not panic (they're tracking methods)
    client.on_transfer(&from, &to, &amount);
    client.on_created(&to, &amount);
    client.on_destroyed(&from, &amount);
}
