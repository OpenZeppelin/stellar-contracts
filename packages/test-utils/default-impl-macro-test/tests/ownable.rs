use soroban_sdk::{
    contract, contractimpl, contracttrait, contracttype, derive_contract, testutils::Address as _,
    Address, Env, String,
};
use stellar_access::Ownable;
use stellar_macros::only_owner;
use stellar_tokens::FungibleToken;

#[contracttype]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct ExampleContract;

#[contracttrait]
impl FungibleToken for ExampleContract {}

#[contracttrait]
impl Ownable for ExampleContract {}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Self::set_owner(e, &owner);
        Base::set_metadata(e, 7, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
    }

    #[only_owner]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        Base::internal_mint(e, &to, amount);
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
