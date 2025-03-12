#![cfg(test)]

use soroban_sdk::{contract, Env, String};

use crate::extensions::metadata::{decimals, name, set_metadata, symbol};

#[contract]
struct MockContract;

#[test]
fn set_and_get_metadata() {
    let e = Env::default();
    let address = e.register(MockContract, ());

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
    let address = e.register(MockContract, ());

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
