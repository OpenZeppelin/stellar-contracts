use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_default_impl_macro::default_impl;
use stellar_non_fungible::{
    enumerable::{Enumerable, NonFungibleEnumerable},
    Balance, Base, NonFungibleToken, TokenId,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env) {
        Base::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Enumerable;
}

#[default_impl]
#[contractimpl]
impl NonFungibleEnumerable for ExampleContract {}

#[contractimpl]
impl ExampleContract {
    pub fn mint(e: &Env, to: Address, token_id: TokenId) {
        Enumerable::non_sequential_mint(e, &to, token_id);
    }

    pub fn burn(e: &Env, from: Address, token_id: TokenId) {
        Enumerable::non_sequential_burn(e, &from, token_id);
    }
}

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_enumerable_total_supply() {
    let e = Env::default();

    let owner = Address::generate(&e);

    let recipient = Address::generate(&e);

    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &10);
    client.transfer(&owner, &recipient, &10);
    assert_eq!(client.total_supply(), 1);
}

#[test]
fn default_impl_enumerable_get_owner_token_id() {
    let e = Env::default();
    let client = create_client(&e);
    let owner = Address::generate(&e);
    e.mock_all_auths();
    client.mint(&owner, &10);
    client.burn(&owner, &10);
    assert_eq!(client.balance(&owner), 0);
    client.mint(&owner, &11);
    assert_eq!(client.balance(&owner), 1);
    assert_eq!(client.get_owner_token_id(&owner, &0), 11);
}

#[test]
fn default_impl_enumerable_get_token_id() {
    let e = Env::default();
    let client = create_client(&e);
    let owner = Address::generate(&e);
    e.mock_all_auths();
    client.mint(&owner, &10);
    client.burn(&owner, &10);
    assert_eq!(client.balance(&owner), 0);
    client.mint(&owner, &11);
    assert_eq!(client.balance(&owner), 1);
    assert_eq!(client.get_token_id(&0), 11);
}
