extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::contract::{IdentityRegistryContract, IdentityRegistryContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> IdentityRegistryContractClient<'a> {
    let address = e.register(IdentityRegistryContract, (admin, manager));
    IdentityRegistryContractClient::new(e, &address)
}

#[test]
fn bind_max() {
    let e = Env::default();
    e.cost_estimate().disable_resource_limits();

    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();

    let mut tokens = vec![&e];
    for _ in 0..200 {
        let token = Address::generate(&e);
        tokens.push_back(token.clone());
    }

    client.bind_tokens(&tokens, &manager);
    assert_eq!(client.linked_tokens().len(), 200)
}
