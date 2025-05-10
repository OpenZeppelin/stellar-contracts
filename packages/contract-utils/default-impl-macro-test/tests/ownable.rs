use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, String,
};
use stellar_default_impl_macro::default_impl;
use stellar_fungible::FungibleToken;
use stellar_ownable::Ownable;
use stellar_ownable_macro::only_owner;

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        stellar_fungible::metadata::set_metadata(
            e,
            7,
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    #[only_owner]
    pub fn mint(e: &Env, caller: Address, to: Address, amount: i128) {
        stellar_fungible::mintable::mint(e, &to, amount);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {}

#[default_impl]
#[contractimpl]
impl Ownable for ExampleContract {}

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_ownable() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &owner, &100);
}
