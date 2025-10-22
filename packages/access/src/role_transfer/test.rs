extern crate std;

use soroban_sdk::{
    contract, contracttype,
    testutils::{Address as _, Ledger},
    Address, Env,
};

use crate::role_transfer::{accept_transfer, transfer_role};

#[contract]
struct MockContract;

#[contracttype]
pub enum MockRole {
    Admin,
    PendingAdmin,
}

#[test]
fn role_transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let active_key = MockRole::Admin;
    let pending_key = MockRole::PendingAdmin;

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&active_key, &admin);

        // Start transfer
        transfer_role(&e, &new_admin, &pending_key, 1000);

        // Accept admin transfer
        accept_transfer(&e, &active_key, &pending_key);

        // Verify new admin
        assert_eq!(e.storage().instance().get::<_, Address>(&MockRole::Admin), Some(new_admin));
    });
}

#[test]
fn role_transfer_cancel_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let pending_key = MockRole::PendingAdmin;

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&MockRole::Admin, &admin);

        // Start admin transfer
        transfer_role(&e, &new_admin, &pending_key, 1000);
    });

    e.as_contract(&address, || {
        // Cancel admin transfer
        transfer_role(&e, &new_admin, &pending_key, 0);

        // Verify admin hasn't changed
        assert_eq!(e.storage().instance().get::<_, Address>(&MockRole::Admin), Some(admin));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2200)")]
fn accept_transfer_with_no_pending_transfer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let active_key = MockRole::Admin;
    let pending_key = MockRole::PendingAdmin;

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&MockRole::Admin, &admin);

        // Attempt to accept transfer with no pending transfer
        accept_transfer(&e, &active_key, &pending_key);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2202)")]
fn cannot_cancel_with_invalid_pending_address() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let wrong_new_admin = Address::generate(&e);
    let pending_key = MockRole::PendingAdmin;

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&MockRole::Admin, &admin);

        // Start admin transfer
        transfer_role(&e, &new_admin, &pending_key, 1000);
    });

    e.as_contract(&address, || {
        // Cancel the transfer with an invalid pending address
        transfer_role(&e, &wrong_new_admin, &pending_key, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2201)")]
fn transfer_with_invalid_live_until_ledger_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let pending_key = MockRole::PendingAdmin;

    e.ledger().set_sequence_number(1000);

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&MockRole::Admin, &admin);

        // Start admin transfer
        transfer_role(&e, &new_admin, &pending_key, 3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2200)")]
fn cancel_transfer_when_there_is_no_pending_transfer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let pending_key = MockRole::PendingAdmin;

    e.as_contract(&address, || {
        // Initialize admin
        e.storage().instance().set(&MockRole::Admin, &admin);

        // Cancel admin transfer when there is no pending transfer
        transfer_role(&e, &new_admin, &pending_key, 0);
    });
}
