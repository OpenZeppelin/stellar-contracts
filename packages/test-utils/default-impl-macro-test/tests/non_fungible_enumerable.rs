use soroban_sdk::{
    contract, contractimpl, contracttrait, testutils::Address as _, Address, Env, String,
};
use stellar_tokens::{
    non_fungible::enumerable::Enumerable, NonFungibleEnumerable, NonFungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contracttrait(default = Enumerable)]
impl NonFungibleToken for ExampleContract {}

#[contracttrait]
impl NonFungibleEnumerable for ExampleContract {}

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

    pub fn mint(e: &Env, to: Address, token_id: u32) {
        Enumerable::non_sequential_mint(e, &to, token_id);
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
