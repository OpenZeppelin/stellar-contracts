#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract, symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env, Symbol,
};
use stellar_event_assertion::EventAssertion;

use crate::{
    accept_admin_transfer, add_to_role_enumeration, get_admin, get_role_admin, get_role_member,
    get_role_member_count, grant_role, has_role, remove_from_role_enumeration, renounce_role,
    revoke_role, set_role_admin, transfer_admin_role,
};

#[contract]
struct MockContract;

const ADMIN_ROLE: Symbol = symbol_short!("admin");
const USER_ROLE: Symbol = symbol_short!("user");
const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[test]
fn admin_transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&crate::AccessControlStorageKey::Admin, &admin);

        // Start admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 1000);

        // Accept admin transfer
        accept_admin_transfer(&e, &new_admin);

        // Verify new admin
        assert_eq!(get_admin(&e), new_admin);

        // Verify events
        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
    });
}

#[test]
fn admin_transfer_cancel_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&crate::AccessControlStorageKey::Admin, &admin);

        // Start admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 1000);

        // Verify events
        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
    });

    e.as_contract(&address, || {
        // Cancel admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 0);

        // Verify admin hasn't changed
        assert_eq!(get_admin(&e), admin);

        // Verify events
        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #123)")]
fn accept_transfer_with_no_pending_transfer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&crate::AccessControlStorageKey::Admin, &admin);

        // Attempt to accept transfer with no pending transfer
        accept_admin_transfer(&e, &new_admin);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #125)")]
fn transfer_with_invalid_live_until_ledger_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    e.ledger().set_sequence_number(1000);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&crate::AccessControlStorageKey::Admin, &admin);

        // Start admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #123)")]
fn cancel_transfer_when_there_is_no_pending_transfer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&crate::AccessControlStorageKey::Admin, &admin);

        // Cancel admin transfer when there is no pending transfer
        transfer_admin_role(&e, &admin, &new_admin, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #120)")]
fn wrong_pending_admin_accept_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let wrong_admin = Address::generate(&e);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&crate::AccessControlStorageKey::Admin, &admin);

        // Start admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 1000);

        // Wrong account attempts to accept transfer
        accept_admin_transfer(&e, &wrong_admin);
    });
}
