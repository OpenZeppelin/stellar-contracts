use soroban_sdk::{
    contract, contractimpl, contracttrait, testutils::Address as _, Address, Env, String,
};
use stellar_tokens::FungibleToken;

#[contract]
pub struct ExampleContract;

#[contracttrait]
impl FungibleToken for ExampleContract {}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env) {
        Self::set_metadata(e, 7, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
    }

    pub fn mint(e: &Env, to: Address, amount: i128) {
        Self::internal_mint(e, &to, amount);
    }
}

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_fungible_total_supply() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    assert_eq!(client.total_supply(), 100);
}

#[test]
fn default_impl_fungible_balance() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    assert_eq!(client.balance(&owner), 100);
}

#[test]
fn default_impl_fungible_transfer() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    client.transfer(&owner, &recipient, &50);
    assert_eq!(client.balance(&owner), 50);
    assert_eq!(client.balance(&recipient), 50);
}

#[test]
fn default_impl_fungible_transfer_from() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    client.approve(&owner, &spender, &50, &1000);
    client.transfer_from(&spender, &owner, &recipient, &30);
    assert_eq!(client.balance(&owner), 70);
    assert_eq!(client.balance(&recipient), 30);
}

#[test]
fn default_impl_fungible_allowance() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    client.approve(&owner, &spender, &50, &1000);
    client.transfer_from(&spender, &owner, &recipient, &30);
    assert_eq!(client.balance(&owner), 70);
    assert_eq!(client.balance(&recipient), 30);
    assert_eq!(client.allowance(&owner, &spender), 20);
}

#[test]
fn default_impl_fungible_approve() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    client.approve(&owner, &spender, &50, &1000);
    assert_eq!(client.allowance(&owner, &spender), 50);
}

#[test]
fn default_impl_fungible_metadata() {
    let e = Env::default();
    let client = create_client(&e);

    assert_eq!(client.name(), String::from_str(&e, "My Token"));
    assert_eq!(client.symbol(), String::from_str(&e, "TKN"));
    assert_eq!(client.decimals(), 7);
}
