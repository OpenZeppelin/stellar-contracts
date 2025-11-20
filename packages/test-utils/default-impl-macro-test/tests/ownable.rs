use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, String,
};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_macros::{default_impl, only_owner};
use stellar_tokens::fungible::{Base, FungibleToken};

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, name: String, symbol: String, owner: Address) {
        set_owner(e, &owner);
        Base::set_metadata(e, 7, name, symbol);
    }

    #[only_owner]
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
impl Ownable for ExampleContract {}

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (name, symbol, owner));
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_ownable() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&owner, &100);
}
