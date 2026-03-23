extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
    initial_supply: &i128,
) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "AllowList Token");
    let symbol = String::from_str(e, "ALT");
    let address = e.register(ExampleContract, (name, symbol, admin, manager, initial_supply));
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

#[test]
fn burn_allowed_account_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let burn_amount = 500;

    e.mock_all_auths();

    // Allow user1 and transfer some tokens to them
    client.allow_user(&user1, &manager);
    client.transfer(&admin, &user1, &1000);
    assert_eq!(client.balance(&user1), 1000);

    // Allowed user can burn their own tokens
    client.burn(&user1, &burn_amount);
    assert_eq!(client.balance(&user1), 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_burn_disallowed_account() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);

    e.mock_all_auths();

    // Allow user1, transfer tokens, then disallow
    client.allow_user(&user1, &manager);
    client.transfer(&admin, &user1, &1000);
    client.disallow_user(&user1, &manager);
    assert!(!client.allowed(&user1));

    // Disallowed user cannot burn
    client.burn(&user1, &500);
}

#[test]
fn burn_from_allowed_account_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let burn_amount = 500;

    e.mock_all_auths();

    // Allow user1 and transfer tokens to them
    client.allow_user(&user1, &manager);
    client.transfer(&admin, &user1, &1000);

    // User1 approves user2 to spend tokens
    client.approve(&user1, &user2, &burn_amount, &1000);

    // User2 can burn from allowed user1's account
    client.burn_from(&user2, &user1, &burn_amount);
    assert_eq!(client.balance(&user1), 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_burn_from_disallowed_account() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);

    e.mock_all_auths();

    // Allow user1, transfer tokens, approve user2, then disallow user1
    client.allow_user(&user1, &manager);
    client.transfer(&admin, &user1, &1000);
    client.approve(&user1, &user2, &500, &1000);
    client.disallow_user(&user1, &manager);
    assert!(!client.allowed(&user1));

    // User2 cannot burn from disallowed user1's account
    client.burn_from(&user2, &user1, &500);
}
