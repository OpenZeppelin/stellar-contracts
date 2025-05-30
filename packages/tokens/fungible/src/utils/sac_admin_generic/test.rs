#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    contract,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, Symbol, Val,
};

use soroban_test_helpers;

use crate::sac_admin_generic::storage::SACAdminGenericDataKey;

use super::{extract_sac_contract_context, get_sac_address, set_sac_address, SacFn};

#[contract]
struct MockContract;

#[soroban_test_helpers::test]
fn test_set_and_get_sac_address(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let sac_addr: Address = e.storage().instance().get(&SACAdminGenericDataKey::Sac).unwrap();
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
