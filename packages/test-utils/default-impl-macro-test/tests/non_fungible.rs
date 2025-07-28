use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::non_fungible::{Base, NonFungibleToken};

#[contract]
pub struct ExampleContract;


#[contracttrait]
impl NonFungibleToken for ExampleContract {} ,
#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env) {
        Base::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com/"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    pub fn mint(e: &Env, to: Address, token_id: u32) {
        Base::internal_mint(e, &to, token_id);
    }
}

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_non_fungible_balance() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    assert_eq!(client.balance(&owner), 1);
}

#[test]
fn default_impl_non_fungible_owner_of() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    assert_eq!(client.owner_of(&10), owner);
}

#[test]
fn default_impl_non_fungible_transfer() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    client.transfer(&owner, &recipient, &10);
    assert_eq!(client.balance(&owner), 0);
    assert_eq!(client.balance(&recipient), 1);
    assert_eq!(client.owner_of(&10), recipient);
}

#[test]
fn default_impl_non_fungible_transfer_from() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    client.approve(&owner, &spender, &10, &1000);
    client.transfer_from(&spender, &owner, &recipient, &10);
    assert_eq!(client.balance(&owner), 0);
    assert_eq!(client.balance(&recipient), 1);
    assert_eq!(client.owner_of(&10), recipient);
}

#[test]
fn default_impl_non_fungible_approve() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let approved = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    client.approve(&owner, &approved, &10, &1000);
    assert_eq!(client.get_approved(&10), Some(approved));
}

#[test]
fn default_impl_non_fungible_approve_for_all() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.approve_for_all(&owner, &operator, &1000);
    assert!(client.is_approved_for_all(&owner, &operator));
}

#[test]
fn default_impl_non_fungible_metadata() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    client.mint(&owner, &1);
    assert_eq!(client.token_uri(&1), String::from_str(&e, "www.mytoken.com/1"));
    assert_eq!(client.name(), String::from_str(&e, "My Token"));
    assert_eq!(client.symbol(), String::from_str(&e, "TKN"));
}
