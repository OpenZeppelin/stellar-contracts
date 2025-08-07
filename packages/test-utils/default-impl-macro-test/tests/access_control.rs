use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, String, Symbol,
};
use stellar_access::{AccessControl, AccessControler};
use stellar_macros::has_role;
use stellar_tokens::{FTBase, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl FungibleToken for ExampleContract {
    type Impl = FTBase;
}

#[contractimpl]
impl AccessControl for ExampleContract {
    type Impl = AccessControler;
}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Self::init_admin(e, &owner);
        Self::set_metadata(e, 7, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
    }

    #[has_role(caller, "minter")]
    pub fn mint(e: &Env, caller: Address, to: Address, amount: i128) {
        Self::internal_mint(e, &to, amount);
    }
}

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_fungible_grant_role() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.grant_role(&owner, &owner, &Symbol::new(&e, "minter"));
}
