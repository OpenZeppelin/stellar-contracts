use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_tokens::{FTBase, FungibleBurnable, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl FungibleToken for ExampleContract {
    type Impl = FTBase;
}

#[contractimpl]
impl FungibleBurnable for ExampleContract {
    type Impl = FTBase;
}

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
