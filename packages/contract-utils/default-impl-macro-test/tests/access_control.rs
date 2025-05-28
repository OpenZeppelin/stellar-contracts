use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, String, Symbol,
};
use stellar_access_control::{set_admin, AccessControl};
use stellar_access_control_macros::has_role;
use stellar_default_impl_macro::default_impl;
use stellar_fungible::FungibleToken;

#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        set_admin(e, &owner);
        stellar_fungible::metadata::set_metadata(
            e,
            7,
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    #[has_role(caller, "minter")]
    pub fn mint(e: &Env, caller: Address, to: Address, amount: i128) {
        stellar_fungible::mintable::mint(e, &to, amount);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}

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
