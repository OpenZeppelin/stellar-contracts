#![cfg(test)]

extern crate std;

use soroban_sdk::Env;

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn add_works() {
    let e = Env::default();
    let client = create_client(&e);

    assert_eq!(client.add(&1, &2), 3);
}
