use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::fungible::{burnable::FungibleBurnable, Base, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, name: String, symbol: String) {
        Base::set_metadata(e, 7, name, symbol);
    }

    pub fn mint(e: &Env, to: Address, amount: i128) {
        Base::mint(e, &to, amount);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = Base;
}

#[default_impl]
#[contractimpl]
impl FungibleBurnable for ExampleContract {}

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (name, symbol));
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_fungible_burnable_burn() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    client.burn(&owner, &50);
    assert_eq!(client.balance(&owner), 50);
    assert_eq!(client.total_supply(), 50);
}

#[test]
fn default_impl_fungible_burnable_burn_from() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner, &100);
    client.approve(&owner, &spender, &50, &1000);
    client.burn_from(&spender, &owner, &30);
    assert_eq!(client.balance(&owner), 70);
    assert_eq!(client.total_supply(), 70);
    assert_eq!(client.allowance(&owner, &spender), 20);
}
