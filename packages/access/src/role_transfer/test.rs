extern crate std;

use soroban_sdk::{
    contract, contracttype,
    testutils::{Address as _, Ledger},
    Address, Env,
};

use crate::role_transfer::{
    accept_transfer, has_active_pending_transfer, transfer_role, PendingTransfer,
};

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

        // Verify no pending transfer remains
        assert!(e.storage().temporary().get::<_, PendingTransfer>(&pending_key).is_none());
        // Verify admin hasn't changed
        assert_eq!(e.storage().instance().get::<_, Address>(&MockRole::Admin), Some(admin));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2203)")]
fn accept_transfer_after_expiry_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let active_key = MockRole::Admin;
    let pending_key = MockRole::PendingAdmin;

    e.ledger().set_sequence_number(2000);

    e.as_contract(&address, || {
        e.storage().instance().set(&active_key, &admin);

        // Directly write a PendingTransfer whose live_until_ledger is in the
        // past relative to the current ledger (2000). This decouples the
        // explicit deadline from the storage TTL, so the entry is present but
        // already expired per our enforced check.
        let expired = PendingTransfer { address: new_admin.clone(), live_until_ledger: 1999 };
        e.storage().temporary().set(&pending_key, &expired);

        accept_transfer(&e, &active_key, &pending_key);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2203)")]
fn accept_transfer_one_ledger_past_deadline_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let active_key = MockRole::Admin;
    let pending_key = MockRole::PendingAdmin;

    e.ledger().set_sequence_number(500);

    e.as_contract(&address, || {
        e.storage().instance().set(&active_key, &admin);

        // Transfer with deadline at ledger 1000
        transfer_role(&e, &new_admin, &pending_key, 1000);
    });

    // Advance one ledger past the deadline; directly overwrite the TTL so the
    // entry is still present in storage, isolating the explicit deadline check.
    e.ledger().set_sequence_number(1001);

    e.as_contract(&address, || {
        let expired = PendingTransfer { address: new_admin.clone(), live_until_ledger: 1000 };
        e.storage().temporary().set(&pending_key, &expired);

        accept_transfer(&e, &active_key, &pending_key);
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

/// Verifies that `has_active_pending_transfer` removes an expired entry from
/// storage and returns `false`.
#[test]
fn has_active_pending_transfer_cleans_up_expired_entry() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let new_admin = Address::generate(&e);
    let pending_key = MockRole::PendingAdmin;

    e.ledger().set_sequence_number(200);

    e.as_contract(&address, || {
        // Manually insert a pending transfer that expired at ledger 100.
        // Use a high storage TTL so the entry survives past the deadline.
        let expired = PendingTransfer { address: new_admin.clone(), live_until_ledger: 100 };
        e.storage().temporary().set(&pending_key, &expired);
        e.storage().temporary().extend_ttl(&pending_key, 1000, 1000);

        // The entry is in storage…
        assert!(e.storage().temporary().get::<_, PendingTransfer>(&pending_key).is_some());

        // …but has_active_pending_transfer should treat it as absent.
        assert!(!has_active_pending_transfer(&e, &pending_key));

        // And the stale entry should now be removed from storage.
        assert!(e.storage().temporary().get::<_, PendingTransfer>(&pending_key).is_none());
    });
}

/// Verifies that `has_active_pending_transfer` returns `true` for a
/// non-expired entry and does not remove it.
#[test]
fn has_active_pending_transfer_returns_true_for_active_entry() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let new_admin = Address::generate(&e);
    let pending_key = MockRole::PendingAdmin;

    e.ledger().set_sequence_number(50);

    e.as_contract(&address, || {
        let active = PendingTransfer { address: new_admin.clone(), live_until_ledger: 100 };
        e.storage().temporary().set(&pending_key, &active);
        e.storage().temporary().extend_ttl(&pending_key, 1000, 1000);

        assert!(has_active_pending_transfer(&e, &pending_key));

        // Entry should still be in storage.
        assert!(e.storage().temporary().get::<_, PendingTransfer>(&pending_key).is_some());
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
