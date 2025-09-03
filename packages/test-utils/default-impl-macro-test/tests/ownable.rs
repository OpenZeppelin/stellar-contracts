use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, String,
};
use stellar_access::{Ownable, Owner};
use stellar_macros::only_owner;
use stellar_tokens::fungible::{FTBase, FungibleToken};

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl FungibleToken for ExampleContract {
    type Impl = FTBase;
}

#[contractimpl]
impl Ownable for ExampleContract {
    type Impl = Owner;
}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Self::set_owner(e, &owner);
        Self::set_metadata(e, 7, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
    }

    #[only_owner]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        Self::internal_mint(e, &to, amount);
    }
}

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_ownable() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client: ExampleContractClient<'_> = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &100);
}
