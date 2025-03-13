#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, Env, String};

use crate::extensions::metadata::storage::{
    decimals, name, set_metadata, symbol, Metadata, METADATA_KEY,
};

#[contract]
struct TestToken;

#[contract]
struct MockContract;

#[test]
fn set_and_get_metadata() {
    let e = Env::default();
    let address = e.register(TestToken, ());

    e.as_contract(&address, || {
        let test_decimals: u32 = 7;
        let test_name = String::from_str(&e, "Test Token");
        let test_symbol = String::from_str(&e, "TEST");

        set_metadata(&e, test_decimals, test_name.clone(), test_symbol.clone());

        assert_eq!(decimals(&e), test_decimals);
        assert_eq!(name(&e), test_name);
        assert_eq!(symbol(&e), test_symbol);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #205)")]
fn get_unset_metadata() {
    let e = Env::default();
    let address = e.register(TestToken, ());

    e.as_contract(&address, || {
        decimals(&e);
    });
}

#[test]
fn metadata_update() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        set_metadata(&e, 6, String::from_str(&e, "Initial Name"), String::from_str(&e, "INI"));

        set_metadata(&e, 8, String::from_str(&e, "Updated Name"), String::from_str(&e, "UPD"));

        assert_eq!(decimals(&e), 8);
        assert_eq!(name(&e), String::from_str(&e, "Updated Name"));
        assert_eq!(symbol(&e), String::from_str(&e, "UPD"));
    });
}

#[test]
fn test_metadata_struct() {
    let e = Env::default();
    e.as_contract(&e.register(TestToken, ()), || {
        // Test case 1: Standard values
        let metadata1 = Metadata {
            decimals: 9,
            name: String::from_str(&e, "Test Token"),
            symbol: String::from_str(&e, "TST"),
        };

        // Store and retrieve to verify serialization
        e.storage().instance().set(&METADATA_KEY, &metadata1);
        let retrieved1: Metadata = e.storage().instance().get(&METADATA_KEY).unwrap();

        assert_eq!(retrieved1.decimals, 9);
        assert_eq!(retrieved1.name, String::from_str(&e, "Test Token"));
        assert_eq!(retrieved1.symbol, String::from_str(&e, "TST"));

        // Test case 2: Edge cases - empty strings and maximum decimal value
        let metadata2 = Metadata {
            decimals: u32::MAX,
            name: String::from_str(&e, ""),
            symbol: String::from_str(&e, ""),
        };

        // Store and retrieve edge case metadata
        e.storage().instance().set(&METADATA_KEY, &metadata2);
        let retrieved2: Metadata = e.storage().instance().get(&METADATA_KEY).unwrap();

        assert_eq!(retrieved2.decimals, u32::MAX);
        assert_eq!(retrieved2.name, String::from_str(&e, ""));
        assert_eq!(retrieved2.symbol, String::from_str(&e, ""));

        // Test case 3: Long strings
        let long_name =
            String::from_str(&e, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let long_symbol = String::from_str(&e, "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");

        let metadata3 =
            Metadata { decimals: 0, name: long_name.clone(), symbol: long_symbol.clone() };

        // Store and retrieve with long strings
        e.storage().instance().set(&METADATA_KEY, &metadata3);
        let retrieved3: Metadata = e.storage().instance().get(&METADATA_KEY).unwrap();

        assert_eq!(retrieved3.decimals, 0);
        assert_eq!(retrieved3.name, long_name);
        assert_eq!(retrieved3.symbol, long_symbol);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #205)")]
fn test_name_unset_metadata() {
    let e = Env::default();
    let address = e.register(TestToken, ());

    e.as_contract(&address, || {
        // This should panic with UnsetMetadata error
        name(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #205)")]
fn test_symbol_unset_metadata() {
    let e = Env::default();
    let address = e.register(TestToken, ());

    e.as_contract(&address, || {
        // This should panic with UnsetMetadata error
        symbol(&e);
    });
}
