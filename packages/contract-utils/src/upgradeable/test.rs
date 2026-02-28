use soroban_sdk::{contract, Env};

use crate::upgradeable::storage::{get_schema_version, set_schema_version};

#[contract]
struct MockContract;

#[test]
fn get_schema_version_defaults_to_zero() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        assert_eq!(get_schema_version(&e), 0);
    });
}

#[test]
fn set_and_get_schema_version() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        set_schema_version(&e, 2);
        assert_eq!(get_schema_version(&e), 2);
    });
}

#[test]
fn schema_version_can_be_updated() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        set_schema_version(&e, 1);
        set_schema_version(&e, 2);
        assert_eq!(get_schema_version(&e), 2);
    });
}
