#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

use crate::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, admin: &Address, initial_supply: &i128) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (admin, initial_supply));
    ExampleContractClient::new(e, &address)
}

#[test]
fn test_blocklist_functionality() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &initial_supply);

    // Verify initial state - no users are blocked
    assert!(!client.blocked(&user1));
    assert!(!client.blocked(&user2));

    // Admin can transfer to user1 initially
    let transfer_amount = 1000;
    e.mock_all_auths();

    // Block user1
    client.block_user(&user1);
    assert!(client.blocked(&user1));

    // Unblock user1
    client.unblock_user(&user1);
    assert!(!client.blocked(&user1));

    // Admin can transfer to user1 again after unblocking
    client.transfer(&admin, &user1, &100);
    assert_eq!(client.balance(&user1), transfer_amount + 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_unauthorized_block() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &initial_supply);

    // Non-admin tries to block a user (should fail)
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "block_user",
            args: (&user,).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.block_user(&user);
}
