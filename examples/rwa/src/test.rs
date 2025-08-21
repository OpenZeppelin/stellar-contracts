#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::identity_registry_storage::{IdentityRegistryContract, IdentityRegistryContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> IdentityRegistryContractClient<'a> {
    let address = e.register(IdentityRegistryContract, (owner,));
    IdentityRegistryContractClient::new(e, &address)
}

#[test]
fn bind_max() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);
    e.mock_all_auths();

    let mut tokens = vec![&e];
    for _ in 0..200 {
        let token = Address::generate(&e);
        tokens.push_back(token.clone());
    }

    client.bind_tokens(&tokens, &owner);
    assert_eq!(client.linked_tokens().len(), 200)
}
