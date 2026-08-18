extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

const CAP: i128 = 1000;

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner, &CAP));
    ExampleContractClient::new(e, &address)
}

#[test]
fn transfer_between_allowed_users_moves_voting_units() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &600);
    client.allow_user(&user, &owner);

    client.transfer(&owner, &user, &250);
    assert_eq!(client.balance(&owner), 350);
    assert_eq!(client.balance(&user), 250);

    client.delegate(&user, &delegate);
    assert_eq!(client.get_votes(&delegate), 250);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn transfer_to_disallowed_user_panics() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &600);
    client.transfer(&owner, &user, &100);
}

#[test]
fn total_supply_served_from_checkpoints() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    assert_eq!(client.total_supply(), 0);
    client.mint(&owner, &600);
    assert_eq!(client.total_supply(), 600);
    assert_eq!(client.get_total_supply(), 600);
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn mint_above_cap_panics() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &600);
    client.mint(&owner, &500);
}

#[test]
fn burn_decreases_supply_and_delegate_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &600);
    client.delegate(&owner, &delegate);
    assert_eq!(client.get_votes(&delegate), 600);

    client.burn(&owner, &200);
    assert_eq!(client.balance(&owner), 400);
    assert_eq!(client.total_supply(), 400);
    assert_eq!(client.get_votes(&delegate), 400);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn burn_by_disallowed_user_panics() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &600);
    client.allow_user(&user, &owner);
    client.transfer(&owner, &user, &100);
    client.disallow_user(&user, &owner);

    client.burn(&user, &50);
}

#[test]
fn cap_frees_up_after_burn() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &CAP);
    client.burn(&owner, &300);
    client.mint(&owner, &300);
    assert_eq!(client.total_supply(), CAP);
}
