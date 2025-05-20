#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn owner_can_increment() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_auths(&[MockAuth {
        address: &owner,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "increment",
            args: ().into_val(&e),
            sub_invokes: &[],
        },
    }]);

    assert_eq!(client.increment(), 1);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn non_owner_cannot_increment() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let non_owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_auths(&[MockAuth {
        address: &non_owner,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "increment",
            args: ().into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.increment();
}
