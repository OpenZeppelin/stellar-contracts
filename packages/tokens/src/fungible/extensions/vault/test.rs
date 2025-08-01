#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::fungible::vault::Vault;

#[contract]
struct MockContract;

#[test]
fn test_vault_asset_address() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let asset_address = Address::generate(&e);
    e.as_contract(&contract_address, || {
        Vault::set_asset(&e, asset_address.clone());
        let asset_queried = Vault::query_asset(&e);
        assert_eq!(asset_queried, asset_address);
    });
}

// TODO: add more test cases
