extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let uri = String::from_str(e, "www.mytoken.com");
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (uri, name, symbol, owner));
    ExampleContractClient::new(e, &address)
}

#[test]
fn transfer_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.mint(&owner);
    client.transfer(&owner, &recipient, &0);
    assert_eq!(client.balance(&owner), 0);
    assert_eq!(client.balance(&recipient), 1);
}

#[test]
fn burn_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.mint(&owner);
    client.burn(&owner, &0);
    assert_eq!(client.balance(&owner), 0);
}
