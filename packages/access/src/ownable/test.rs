extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::ownable::{
    accept_ownership, enforce_owner_auth, get_owner, renounce_ownership, set_owner,
    transfer_ownership, OwnableStorageKey,
};

#[contract]
struct MockContract;

#[test]
fn transfer_ownership_sets_pending() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let new_owner = Address::generate(&e);
    let contract = e.register(MockContract, ());

    e.mock_all_auths();

    e.as_contract(&contract, || {
        set_owner(&e, &owner);
    });

    e.as_contract(&contract, || {
        transfer_ownership(&e, &new_owner, 1000);

        let pending: Option<Address> =
            e.storage().temporary().get(&OwnableStorageKey::PendingOwner);
        assert_eq!(pending, Some(new_owner));

        let assert = EventAssertion::new(&e, contract.clone());
        assert.assert_event_count(1);
    });
}

#[test]
fn accept_ownership_completes_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let old_owner = Address::generate(&e);
    let new_owner = Address::generate(&e);
    let contract = e.register(MockContract, ());

    e.as_contract(&contract, || {
        set_owner(&e, &old_owner);
        e.storage().temporary().set(&OwnableStorageKey::PendingOwner, &new_owner);

        accept_ownership(&e);

        let stored_owner = get_owner(&e);
        assert_eq!(stored_owner, Some(new_owner));

        let assert = EventAssertion::new(&e, contract.clone());
        assert.assert_event_count(1);
    });
}

#[test]
fn renounce_ownership_removes_owner() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let contract = e.register(MockContract, ());

    e.as_contract(&contract, || {
        set_owner(&e, &owner);
    });

    e.mock_all_auths();

    e.as_contract(&contract, || {
        renounce_ownership(&e);

        assert_eq!(get_owner(&e), None);

        let assert = EventAssertion::new(&e, contract.clone());
        assert.assert_event_count(1);
    });
}

#[test]
fn enforce_owner_auth_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let contract = e.register(MockContract, ());

    e.as_contract(&contract, || {
        set_owner(&e, &owner);
    });

    e.mock_all_auths();

    e.as_contract(&contract, || {
        enforce_owner_auth(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2100)")]
fn enforce_owner_auth_panics_if_renounced() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let contract = e.register(MockContract, ());

    e.as_contract(&contract, || {
        set_owner(&e, &owner);
    });

    e.mock_all_auths();

    e.as_contract(&contract, || {
        renounce_ownership(&e);

        assert_eq!(get_owner(&e), None);
    });

    e.as_contract(&contract, || {
        enforce_owner_auth(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2101)")]
fn renounce_fails_if_pending_transfer_exists() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let pending = Address::generate(&e);
    let contract = e.register(MockContract, ());

    e.as_contract(&contract, || {
        set_owner(&e, &owner);
        e.storage().temporary().set(&OwnableStorageKey::PendingOwner, &pending);
    });

    e.mock_all_auths();

    e.as_contract(&contract, || {
        renounce_ownership(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2102)")]
fn set_owner_when_already_set_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let contract = e.register(MockContract, ());
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);

    e.as_contract(&contract, || {
        // Set owner for the first time - should succeed
        set_owner(&e, &owner1);

        // Verify owner is set correctly
        let current_owner = get_owner(&e).unwrap();
        assert_eq!(current_owner, owner1);

        // Try to set owner again - should panic with OwnerAlreadySet error
        set_owner(&e, &owner2);
    });
}
