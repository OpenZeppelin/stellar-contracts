use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, MuxedAddress,
    String, Symbol,
};
use stellar_access::access_control::{set_admin, AccessControl};
use stellar_macros::{default_impl, has_role};
use stellar_tokens::fungible::{Base, FungibleToken};

#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, name: String, symbol: String, owner: Address) {
        set_admin(e, &owner);
        Base::set_metadata(e, 7, name, symbol);
    }

    #[has_role(caller, "minter")]
    pub fn mint(e: &Env, to: Address, amount: i128, caller: Address) {
        Base::mint(e, &to, amount);
    }
}

#[contractimpl(contracttrait = true)]
impl FungibleToken for ExampleContract {
    type ContractType = Base;
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (name, symbol, owner));
    ExampleContractClient::new(e, &address)
}

#[test]
fn default_impl_fungible_grant_role() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.grant_role(&owner, &Symbol::new(&e, "minter"), &owner);
}
