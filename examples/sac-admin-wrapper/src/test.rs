#![cfg(test)]
extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    Address, Env, IntoVal,
};

use crate::contract::{ExampleContract, ExampleContractClient};

#[test]
fn test_sac_transfer() {
    let e = Env::default();

    let issuer = Address::generate(&e);
    let default_admin = Address::generate(&e);
    let manager1 = Address::generate(&e);
    let manager2 = Address::generate(&e);
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
    let new_admin = e.register(
        ExampleContract,
        (default_admin.clone(), manager1.clone(), manager2, sac_client.address.clone()),
    );
    let new_admin_client = ExampleContractClient::new(&e, &new_admin);

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
        // default_admin authorizes
        address: &manager1,
        invoke: &MockAuthInvoke {
            contract: &new_admin,
            fn_name: "mint",
            args: (&user2, 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    new_admin_client.mint(&user2, &1000, &manager1);

    let balance2 = token_client.balance(&user2);
    assert_eq!(balance2, 1000);
}

#[test]
fn test_transfer_admin() {
    let e = Env::default();

    let issuer = Address::generate(&e);
    let default_admin = Address::generate(&e);
    let new_default_admin = Address::generate(&e);
    let manager1 = Address::generate(&e);
    let manager2 = Address::generate(&e);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());

    // Deploy the New Admin
    let new_admin = e.register(
        ExampleContract,
        (default_admin.clone(), manager1.clone(), manager2, sac_client.address.clone()),
    );
    let new_admin_client = ExampleContractClient::new(&e, &new_admin);

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

    e.mock_auths(&[MockAuth {
        // default_admin authorizes
        address: &manager1,
        invoke: &MockAuthInvoke {
            contract: &new_admin,
            fn_name: "set_admin",
            args: (&new_default_admin, &manager1).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    assert!(new_admin_client.try_set_admin(&new_default_admin, &manager1).is_err());

    e.mock_auths(&[MockAuth {
        // default_admin authorizes
        address: &default_admin,
        invoke: &MockAuthInvoke {
            contract: &new_admin,
            fn_name: "set_admin",
            args: (&new_default_admin, &default_admin).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    new_admin_client.set_admin(&new_default_admin, &default_admin);
    assert_eq!(sac_client.admin(), new_default_admin);
}
