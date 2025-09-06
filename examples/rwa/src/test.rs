extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::identity_registry_storage::{IdentityRegistryContract, IdentityRegistryContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    _manager: &Address,
) -> IdentityRegistryContractClient<'a> {
    let address = e.register(IdentityRegistryContract, (admin,));
    IdentityRegistryContractClient::new(e, &address)
}

#[test]
fn bind_max() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();

    // TODO: remove this and move the constructor arguments to `create_client` when
    // `#[contract_impl]` is updated
    //client.constructor(&admin, &manager);

    for _ in 0..200 {
        let token = Address::generate(&e);
        client.bind_token(&token, &manager);
    }

    assert_eq!(client.linked_tokens().len(), 200)
}

// TODO: add test for checking `recovery_address` fails when contract is paused
