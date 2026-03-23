extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
    initial_supply: &i128,
) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "BlockList Token");
    let symbol = String::from_str(e, "BLT");
    let address = e.register(ExampleContract, (name, symbol, admin, manager, initial_supply));
    ExampleContractClient::new(e, &address)
}

#[test]
fn block_unblock_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);

    // Verify initial state - no users are blocked
    assert!(!client.blocked(&user1));
    assert!(!client.blocked(&user2));

    // Admin can transfer to user1 initially
    let transfer_amount = 1000;
    e.mock_all_auths();

    // Block user1
    client.block_user(&user1, &manager);
    assert!(client.blocked(&user1));

    // Unblock user1
    client.unblock_user(&user1, &manager);
    assert!(!client.blocked(&user1));

    // Admin can transfer to user1 again after unblocking
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_user_cannot_approve() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // Block user1
    client.block_user(&user1, &manager);
    assert!(client.blocked(&user1));

    // Blocked user1 tries to approve user2 (should fail)
    client.approve(&user1, &user2, &transfer_amount, &1000);
}

#[test]
fn blocklist_approve_override_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - no users are blocked
    assert!(!client.blocked(&user1));
    assert!(!client.blocked(&user2));

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // User1 approves user2
    client.approve(&user1, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&user1, &user2), transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn transfer_from_blocked_user() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // User1 approves user2
    client.approve(&user1, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&user1, &user2), transfer_amount);

    // Block user1 (the from account)
    client.block_user(&user1, &manager);
    assert!(client.blocked(&user1));

    // User2 tries to transfer from blocked user1 to user3 (should fail)
    client.transfer_from(&user2, &user1, &user3, &transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn transfer_from_to_blocked_user() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // User1 approves user2
    client.approve(&user1, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&user1, &user2), transfer_amount);

    // Block user3 (the recipient)
    client.block_user(&user3, &manager);
    assert!(client.blocked(&user3));

    // User2 tries to transfer from user1 to blocked user3 (should fail)
    client.transfer_from(&user2, &user1, &user3, &transfer_amount);
}

#[test]
fn blocklist_transfer_from_override_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - no users are blocked
    assert!(!client.blocked(&user1));
    assert!(!client.blocked(&user2));
    assert!(!client.blocked(&user3));

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // User1 approves user2
    client.approve(&user1, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&user1, &user2), transfer_amount);

    // User2 transfers from user1 to user3
    client.transfer_from(&user2, &user1, &user3, &transfer_amount);
    assert_eq!(client.balance(&user3), transfer_amount);
    assert_eq!(client.balance(&user1), 0);
}

#[test]
fn burn_non_blocked_account_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let burn_amount = 500;

    e.mock_all_auths();

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &1000);
    assert_eq!(client.balance(&user1), 1000);

    // Non-blocked user can burn their own tokens
    client.burn(&user1, &burn_amount);
    assert_eq!(client.balance(&user1), 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_user_cannot_burn() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);

    e.mock_all_auths();

    // Transfer some tokens to user1, then block them
    client.transfer(&admin, &user1, &1000);
    client.block_user(&user1, &manager);
    assert!(client.blocked(&user1));

    // Blocked user cannot burn
    client.burn(&user1, &500);
}

#[test]
fn burn_from_non_blocked_account_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let burn_amount = 500;

    e.mock_all_auths();

    // Transfer some tokens to user1
    client.transfer(&admin, &user1, &1000);

    // User1 approves user2 to spend tokens
    client.approve(&user1, &user2, &burn_amount, &1000);

    // User2 can burn from non-blocked user1's account
    client.burn_from(&user2, &user1, &burn_amount);
    assert_eq!(client.balance(&user1), 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn cannot_burn_from_blocked_account() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);

    e.mock_all_auths();

    // Transfer some tokens to user1, approve user2, then block user1
    client.transfer(&admin, &user1, &1000);
    client.approve(&user1, &user2, &500, &1000);
    client.block_user(&user1, &manager);
    assert!(client.blocked(&user1));

    // User2 cannot burn from blocked user1's account
    client.burn_from(&user2, &user1, &500);
}
