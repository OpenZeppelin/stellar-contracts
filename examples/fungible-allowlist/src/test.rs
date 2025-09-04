extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
    initial_supply: &i128,
) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (admin, manager, initial_supply));
    ExampleContractClient::new(e, &address)
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_transfer_before_allow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Admin can't transfer to user1 initially (user1 not allowed)
    e.mock_all_auths();
    client.transfer(&admin, &user1, &transfer_amount);
}

#[test]
fn transfer_to_allowed_account_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1, &manager);
    assert!(client.allowed(&user1));

    // Now admin can transfer to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_transfer_after_disallow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1, &manager);
    assert!(client.allowed(&user1));

    // Now admin can transfer to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // Disallow user1
    client.disallow_user(&user1, &manager);
    assert!(!client.allowed(&user1));

    // Admin can't transfer to user1 after disallowing
    client.transfer(&admin, &user1, &100);
}

#[test]
fn allowlist_transfer_from_override_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user2
    client.allow_user(&user2, &manager);
    assert!(client.allowed(&user2));

    // Now admin can transfer to user1
    client.approve(&admin, &user1, &transfer_amount, &1000);
    client.transfer_from(&user1, &admin, &user2, &transfer_amount);
    assert_eq!(client.balance(&user2), transfer_amount);
}

#[test]
fn allowlist_approve_override_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1, &manager);
    assert!(client.allowed(&user1));

    // Approve user2 to transfer from user1
    client.approve(&user1, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&user1, &user2), transfer_amount);
}
