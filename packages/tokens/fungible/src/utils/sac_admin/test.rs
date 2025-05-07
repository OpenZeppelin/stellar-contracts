#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, MockAuth, MockAuthInvoke, StellarAssetContract},
    token::StellarAssetClient,
    Address, Env, IntoVal,
};
use soroban_test_helpers;

use crate::sac_admin::storage::{get_sac_address, set_sac_address};

use super::storage::set_admin;

#[contract]
struct MockContract;

#[soroban_test_helpers::test]
fn test_sac_set_address(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        assert_eq!(get_sac_address(&e), sac);
    });
}

#[soroban_test_helpers::test]
#[should_panic(expected = "Error(Contract, #209)")]
fn test_sac_get_address_fails(e: Env) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || get_sac_address(&e));
}

#[soroban_test_helpers::test]
fn test_sac_set_admin(e: Env, issuer: Address) {
    let new_admin = e.register(MockContract, ());

    // Deploy the Stellar Asset Contract
    let sac: StellarAssetContract = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());
    assert_eq!(sac_client.admin(), issuer);

    env.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &issuer,
        invoke: &MockAuthInvoke {
            contract: &sac_client.address,
            fn_name: "set_admin",
            args: (&new_admin,).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac.address());
        set_admin(&e, &new_admin);
        assert_eq!(sac_client.admin(), new_admin);
    });
}
