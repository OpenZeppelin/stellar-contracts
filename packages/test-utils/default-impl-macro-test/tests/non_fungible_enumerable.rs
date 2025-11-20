use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::non_fungible::{
    enumerable::{Enumerable, NonFungibleEnumerable},
    Base, NonFungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, uri: String, name: String, symbol: String) {
        Base::set_metadata(e, uri, name, symbol);
    }

    pub fn mint(e: &Env, to: Address, token_id: u32) {
        Enumerable::non_sequential_mint(e, &to, token_id);
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

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let uri = String::from_str(e, "www.mytoken.com");
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (uri, name, symbol));
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
