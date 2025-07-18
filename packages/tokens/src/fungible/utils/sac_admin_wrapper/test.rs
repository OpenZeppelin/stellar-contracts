#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, MockAuth, MockAuthInvoke, StellarAssetContract},
    token::{StellarAssetClient, TokenClient},
    xdr::AccountFlags,
    Address, Env, IntoVal,
};
use soroban_test_helpers;

use crate::sac_admin_wrapper::storage::{
    clawback, get_sac_address, mint, set_admin, set_authorized, set_sac_address,
    SACAdminWrapperDataKey,
};

fn create_sac_client<'a>(e: &Env, issuer: &Address) -> StellarAssetClient<'a> {
    let sac: StellarAssetContract = e.register_stellar_asset_contract_v2(issuer.clone());
    // set flag so that `set_authorized()` can be tested
    sac.issuer().set_flag(AccountFlags::RevocableFlag);
    // set flag so that `clawback()` can be tested
    sac.issuer().set_flag(AccountFlags::ClawbackEnabledFlag);
    let sac_client = StellarAssetClient::new(e, &sac.address());
    assert_eq!(sac_client.admin(), issuer.clone());

    sac_client
}

fn mock_issuer_sets_new_admin(e: &Env, issuer: &Address, sac: &Address, new_admin: &Address) {
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: issuer,
        invoke: &MockAuthInvoke {
            contract: sac,
            fn_name: "set_admin",
            args: (new_admin,).into_val(e),
            sub_invokes: &[],
        },
    }]);
    e.as_contract(new_admin, || {
        set_sac_address(e, sac);
        set_admin(e, new_admin);
    });
}

#[contract]
struct MockContract;

#[soroban_test_helpers::test]
fn test_sac_set_address(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let sac_addr: Address = e.storage().instance().get(&SACAdminWrapperDataKey::Sac).unwrap();
        assert_eq!(get_sac_address(&e), sac);
        assert_eq!(sac_addr, sac);
    });
}

#[soroban_test_helpers::test]
#[should_panic(expected = "Error(Contract, #109)")]
fn test_sac_get_address_fails(e: Env) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || get_sac_address(&e));
}

#[soroban_test_helpers::test]
fn test_sac_set_admin(e: Env, issuer: Address) {
    let new_admin = e.register(MockContract, ());
    let sac_client = create_sac_client(&e, &issuer);

    mock_issuer_sets_new_admin(&e, &issuer, &sac_client.address, &new_admin);
    assert_eq!(sac_client.admin(), new_admin);
}

#[soroban_test_helpers::test]
fn test_sac_mint(e: Env, issuer: Address, user: Address) {
    let new_admin = e.register(MockContract, ());
    let sac_client = create_sac_client(&e, &issuer);

    mock_issuer_sets_new_admin(&e, &issuer, &sac_client.address, &new_admin);
    let token_client = TokenClient::new(&e, &sac_client.address);
    e.as_contract(&new_admin, || {
        mint(&e, &user, 100);
        assert_eq!(token_client.balance(&user), 100);
    });
}

#[soroban_test_helpers::test]
// This error is emitted by the host environment and
// is only indirectly related to this module.
//
// For a reference, the following is part of its trace:
// topics:[error, Error(Contract, #11)], data:"balance is deauthorized"
#[should_panic(expected = "Error(Contract, #11)")]
fn test_sac_set_authorized(e: Env, issuer: Address, user: Address, other: Address) {
    let new_admin = e.register(MockContract, ());
    let sac_client = create_sac_client(&e, &issuer);

    mock_issuer_sets_new_admin(&e, &issuer, &sac_client.address, &new_admin);

    e.as_contract(&new_admin, || {
        mint(&e, &user, 100);
        set_authorized(&e, &other, false);
    });

    let token_client = TokenClient::new(&e, &sac_client.address);
    e.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &sac_client.address,
            fn_name: "transfer",
            args: (&user, &other, 100i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    token_client.transfer(&user, &other, &100)
}

#[soroban_test_helpers::test]
fn test_sac_clawback(e: Env, issuer: Address, user: Address) {
    let new_admin = e.register(MockContract, ());
    let sac_client = create_sac_client(&e, &issuer);

    mock_issuer_sets_new_admin(&e, &issuer, &sac_client.address, &new_admin);

    e.as_contract(&new_admin, || {
        mint(&e, &user, 100);
        clawback(&e, &user, 50);
    });

    let token_client = TokenClient::new(&e, &sac_client.address);
    assert_eq!(token_client.balance(&user), 50);
}
