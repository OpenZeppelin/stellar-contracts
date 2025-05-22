#![cfg(test)]
extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    Address, Env, IntoVal,
};

use crate::contract::{ExampleSACAdminWrapperContract, ExampleSACAdminWrapperContractClient};

#[test]
fn test_sac_transfer() {
    let e = Env::default();

    let issuer = Address::generate(&e);
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());

    // Mint 1000 tokens to user1 from the SAC
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &issuer,
        invoke: &MockAuthInvoke {
            contract: &sac_client.address,
            fn_name: "mint",
            args: (&user1, 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    sac_client.mint(&user1, &1000);

    let token_client = TokenClient::new(&e, &sac.address());

    let balance1 = token_client.balance(&user1);
    assert_eq!(balance1, 1000);

    // Deploy the New Admin
    let new_admin =
        e.register(ExampleSACAdminWrapperContract, (owner.clone(), sac_client.address.clone()));
    let new_admin_client = ExampleSACAdminWrapperContractClient::new(&e, &new_admin);

    // Set the New Admin
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &issuer,
        invoke: &MockAuthInvoke {
            contract: &sac_client.address,
            fn_name: "set_admin",
            args: (&new_admin,).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    sac_client.set_admin(&new_admin);
    assert_eq!(sac_client.admin(), new_admin);

    // Mint 1000 tokens to user2 from the New Admin
    e.mock_auths(&[MockAuth {
        // owner authorizes
        address: &owner,
        invoke: &MockAuthInvoke {
            contract: &new_admin,
            fn_name: "mint",
            args: (&user2, 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    new_admin_client.mint(&user2, &1000, &owner);

    let balance2 = token_client.balance(&user2);
    assert_eq!(balance2, 1000);
}
