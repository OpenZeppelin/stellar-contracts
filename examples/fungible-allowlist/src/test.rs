#![cfg(test)]

extern crate std;

use crate::{ExampleContract, ExampleContractClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

fn create_client<'a>(e: &Env, admin: &Address, initial_supply: &i128) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (admin, initial_supply));
    ExampleContractClient::new(e, &address)
}

#[test]
#[should_panic(expected = "Error(Contract, #209)")]
fn cannot_transfer_before_allow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &initial_supply);
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
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &initial_supply);
    let transfer_amount = 1000;

    // e.mock_all_auths();

    // // Verify initial state - admin is allowed, others are not
    // assert!(!client.allowed(&admin));
    // assert!(!client.allowed(&user1));
    // assert!(!client.allowed(&user2));

    // // Allow user1
    // client.allow_user(&user1);
    // assert!(client.allowed(&user1));

    // // Now admin can transfer to user1
    // client.transfer(&admin, &user1, &transfer_amount);
    // assert_eq!(client.balance(&user1), transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #209)")]
fn cannot_transfer_after_disallow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &initial_supply);
    let transfer_amount = 1000;

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1);
    assert!(client.allowed(&user1));

    // Now admin can transfer to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // Disallow user1
    client.disallow_user(&user1);
    assert!(!client.allowed(&user1));

    // Admin can't transfer to user1 after disallowing
    client.transfer(&admin, &user1, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_unauthorized_allow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &initial_supply);

    // Non-admin tries to allow a user (should fail)
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "allow_user",
            args: (&user,).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.allow_user(&user);
}
