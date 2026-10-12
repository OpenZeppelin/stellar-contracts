extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, Event,
};
use stellar_tokens::fungible::allowlist::{UserAllowed, UserDisallowed};

use crate::contract::{AllowlistPolicyContract, AllowlistPolicyContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> AllowlistPolicyContractClient<'a> {
    let address = e.register(AllowlistPolicyContract, (admin, manager));
    AllowlistPolicyContractClient::new(e, &address)
}

#[test]
fn allow_and_disallow_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let alice = Address::generate(&e);

    assert!(!client.allowed(&alice));

    client.allow(&alice, &manager);
    let events = e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &UserAllowed { user: alice.clone() }.to_xdr(&e, &client.address)
    );
    assert!(client.allowed(&alice));

    client.disallow(&alice, &manager);
    let events = e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &UserDisallowed { user: alice.clone() }.to_xdr(&e, &client.address)
    );
    assert!(!client.allowed(&alice));
}

#[test]
fn repeated_allow_and_disallow_are_no_ops() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    client.allow(&alice, &manager);
    client.allow(&alice, &manager);
    assert_eq!(e.events().all().events().len(), 0);
    assert!(client.allowed(&alice));

    client.disallow(&alice, &manager);
    client.disallow(&alice, &manager);
    assert_eq!(e.events().all().events().len(), 0);
    assert!(!client.allowed(&alice));

    client.disallow(&bob, &manager);
    assert_eq!(e.events().all().events().len(), 0);
    assert!(!client.allowed(&bob));
}

#[test]
fn is_authorized_follows_membership_for_every_token() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);

    assert!(!client.is_authorized(&alice, &token_a));

    client.allow(&alice, &manager);
    assert!(client.is_authorized(&alice, &token_a));
    assert!(client.is_authorized(&alice, &token_b));
    assert!(!client.is_authorized(&bob, &token_a));

    client.disallow(&alice, &manager);
    assert!(!client.is_authorized(&alice, &token_a));
    assert!(!client.is_authorized(&alice, &token_b));
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn allow_by_non_manager_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.allow(&Address::generate(&e), &admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn disallow_by_non_manager_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let alice = Address::generate(&e);
    client.allow(&alice, &manager);

    client.disallow(&alice, &Address::generate(&e));
}
