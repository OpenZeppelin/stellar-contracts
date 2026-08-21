extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn transfer_moves_balance_and_voting_units() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &500);
    client.transfer(&owner, &user, &200);
    assert_eq!(client.balance(&owner), 300);
    assert_eq!(client.balance(&user), 200);

    client.delegate(&user, &delegate);
    assert_eq!(client.get_votes(&delegate), 200);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_small_holder_cannot_transfer() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &100);
    client.block_user(&user);
    client.transfer(&user, &owner, &10);
}

#[test]
fn blocked_whale_is_exempt_until_drained() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &500);
    client.block_user(&user);
    assert!(client.blocked(&user));

    // Holding more than 100, the blocked account can still transfer.
    client.transfer(&user, &owner, &450);
    assert_eq!(client.balance(&user), 50);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_whale_frozen_after_dropping_below_threshold() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &500);
    client.block_user(&user);
    client.transfer(&user, &owner, &450);

    // Now at 50 (<= threshold), the block applies.
    client.transfer(&user, &owner, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_empty_recipient_cannot_receive() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &500);
    client.block_user(&user);
    client.transfer(&owner, &user, &50);
}

#[test]
fn blocked_whale_recipient_can_receive() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &500);
    client.mint(&user, &200);
    client.block_user(&user);

    client.transfer(&owner, &user, &50);
    assert_eq!(client.balance(&user), 250);
}

#[test]
fn burn_respects_whale_rule_and_updates_supply_and_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &500);
    client.delegate(&user, &delegate);
    client.block_user(&user);

    client.burn(&user, &200);
    assert_eq!(client.balance(&user), 300);
    assert_eq!(client.total_supply(), 300);
    assert_eq!(client.get_votes(&delegate), 300);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_small_holder_cannot_burn() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &50);
    client.block_user(&user);
    client.burn(&user, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocked_small_holder_cannot_approve() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &50);
    client.block_user(&user);
    client.approve(&user, &spender, &10, &100);
}

#[test]
fn total_supply_served_from_checkpoints() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    assert_eq!(client.total_supply(), 0);
    client.mint(&owner, &500);
    assert_eq!(client.total_supply(), 500);
    assert_eq!(client.get_total_supply(), 500);

    client.burn(&owner, &100);
    assert_eq!(client.total_supply(), 400);
    assert_eq!(client.get_total_supply(), 400);
}

#[test]
fn unblock_restores_small_holder() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user, &50);
    client.block_user(&user);
    client.unblock_user(&user);
    assert!(!client.blocked(&user));

    client.transfer(&user, &owner, &10);
    assert_eq!(client.balance(&user), 40);
}
