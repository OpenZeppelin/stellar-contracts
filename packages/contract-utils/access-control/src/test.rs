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
    revoke_role, set_admin, set_role_admin, transfer_admin_role,
};

#[contract]
struct MockContract;

const ADMIN_ROLE: Symbol = symbol_short!("admin");
const USER_ROLE: Symbol = symbol_short!("user");
const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[test]
fn admin_functions_work() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Admin can grant roles
        grant_role(&e, &admin, &user, &USER_ROLE);
        assert!(has_role(&e, &user, &USER_ROLE).is_some());

        // Test events
        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
    });

    e.as_contract(&address, || {
        // Admin can revoke roles
        revoke_role(&e, &admin, &user, &USER_ROLE);
        assert!(has_role(&e, &user, &USER_ROLE).is_none());

        // Test events
        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
    });
}

#[test]
fn role_management_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Grant roles to multiple users
        grant_role(&e, &admin, &user1, &USER_ROLE);
    });

    e.as_contract(&address, || {
        grant_role(&e, &admin, &user2, &USER_ROLE);

        // Check role count
        assert_eq!(get_role_member_count(&e, &USER_ROLE), 2);

        // Check role members
        assert_eq!(get_role_member(&e, &USER_ROLE, 0), user1);
        assert_eq!(get_role_member(&e, &USER_ROLE, 1), user2);
    });

    e.as_contract(&address, || {
        // Revoke role from first user
        revoke_role(&e, &admin, &user1, &USER_ROLE);

        // Check updated count and enumeration
        assert_eq!(get_role_member_count(&e, &USER_ROLE), 1);
        assert_eq!(get_role_member(&e, &USER_ROLE, 0), user2);
    });
}

#[test]
fn role_admin_management_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Set MANAGER_ROLE as admin for USER_ROLE
        set_role_admin(&e, &admin, &USER_ROLE, &MANAGER_ROLE);
    });

    e.as_contract(&address, || {
        // Grant MANAGER_ROLE to manager
        grant_role(&e, &admin, &manager, &MANAGER_ROLE);

        // Manager can now grant USER_ROLE
        grant_role(&e, &manager, &user, &USER_ROLE);
        assert!(has_role(&e, &user, &USER_ROLE).is_some());
    });

    e.as_contract(&address, || {
        // Manager can revoke USER_ROLE
        revoke_role(&e, &manager, &user, &USER_ROLE);
        assert!(has_role(&e, &user, &USER_ROLE).is_none());
    });
}

#[test]
fn get_role_member_count_for_nonexistent_role_returns_zero() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let nonexistent_role = Symbol::new(&e, "nonexistent");

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Get count for a role that doesn't exist
        let count = get_role_member_count(&e, &nonexistent_role);

        // Should return 0 for non-existent roles
        assert_eq!(count, 0);
    });
}

#[test]
fn get_role_admin_returns_some_when_set() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Set ADMIN_ROLE as the admin for USER_ROLE
        set_role_admin(&e, &admin, &USER_ROLE, &ADMIN_ROLE);

        // Check that get_role_admin returns the correct admin role
        let admin_role = get_role_admin(&e, &USER_ROLE);
        assert_eq!(admin_role, Some(ADMIN_ROLE));
    });
}

#[test]
fn get_role_admin_returns_none_when_not_set() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // No admin role has been set for USER_ROLE

        // Check that get_role_admin returns None
        let admin_role = get_role_admin(&e, &USER_ROLE);
        assert_eq!(admin_role, None);
    });
}

#[test]
fn renounce_role_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Grant role to user
        grant_role(&e, &admin, &user, &USER_ROLE);
        assert!(has_role(&e, &user, &USER_ROLE).is_some());

        // User can renounce their own role
        renounce_role(&e, &user, &USER_ROLE);
        assert!(has_role(&e, &user, &USER_ROLE).is_none());
    });
}

#[test]
fn admin_transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

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
        set_admin(&e, &admin);

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
#[should_panic(expected = "Error(Contract, #120)")]
fn unauthorized_role_grant_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let other = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Unauthorized user attempts to grant role
        grant_role(&e, &other, &user, &USER_ROLE);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #120)")]
fn unauthorized_role_revoke_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let other = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Grant role to user
        grant_role(&e, &admin, &user, &USER_ROLE);

        // Unauthorized user attempts to revoke role
        revoke_role(&e, &other, &user, &USER_ROLE);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #122)")]
fn renounce_nonexistent_role_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // User attempts to renounce role they don't have
        renounce_role(&e, &user, &USER_ROLE);
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
        set_admin(&e, &admin);

        // Attempt to accept transfer with no pending transfer
        accept_admin_transfer(&e, &new_admin);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #124)")]
fn transfer_with_invalid_live_until_ledger_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    e.ledger().set_sequence_number(1000);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

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
        set_admin(&e, &admin);

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
        set_admin(&e, &admin);

        // Start admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 1000);

        // Wrong account attempts to accept transfer
        accept_admin_transfer(&e, &wrong_admin);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #122)")]
fn get_admin_with_no_admin_set_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // No admin is set in storage
        get_admin(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #122)")]
fn get_role_member_with_out_of_bounds_index_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Grant role to create one member
        grant_role(&e, &admin, &user, &USER_ROLE);

        // Verify count is 1
        assert_eq!(get_role_member_count(&e, &USER_ROLE), 1);

        // Try to access index that is out of bounds (only index 0 exists)
        get_role_member(&e, &USER_ROLE, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #120)")]
fn transfer_admin_role_from_non_admin_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let non_admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Non-admin attempts to transfer admin role
        transfer_admin_role(&e, &non_admin, &new_admin, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #120)")]
fn cancel_transfer_admin_role_from_non_admin_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let non_admin = Address::generate(&e);
    let new_admin = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Start a valid admin transfer
        transfer_admin_role(&e, &admin, &new_admin, 1000);

        // Non-admin attempts to cancel the admin transfer
        transfer_admin_role(&e, &non_admin, &new_admin, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #120)")]
fn set_role_admin_from_non_admin_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let admin = Address::generate(&e);
    let non_admin = Address::generate(&e);

    e.as_contract(&address, || {
        set_admin(&e, &admin);

        // Non-admin attempts to set a role admin
        set_role_admin(&e, &non_admin, &USER_ROLE, &MANAGER_ROLE);
    });
}

#[test]
fn add_to_role_enumeration_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        // Initial count should be 0
        let count_before = get_role_member_count(&e, &USER_ROLE);
        assert_eq!(count_before, 0);

        // Directly call the enumeration function
        add_to_role_enumeration(&e, &account, &USER_ROLE);

        // Count should be incremented
        let count_after = get_role_member_count(&e, &USER_ROLE);
        assert_eq!(count_after, 1);

        // Account should be retrievable by index
        let retrieved = get_role_member(&e, &USER_ROLE, 0);
        assert_eq!(retrieved, account);

        // Account should have the role
        let has_role = has_role(&e, &account, &USER_ROLE);
        assert_eq!(has_role, Some(0));
    });
}

#[test]
fn remove_from_role_enumeration_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account1 = Address::generate(&e);
    let account2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add two accounts
        add_to_role_enumeration(&e, &account1, &USER_ROLE);
        add_to_role_enumeration(&e, &account2, &USER_ROLE);

        // Initial count should be 2
        let count_before = get_role_member_count(&e, &USER_ROLE);
        assert_eq!(count_before, 2);

        // Directly call the removal function
        remove_from_role_enumeration(&e, &account1, &USER_ROLE);

        // Count should be decremented
        let count_after = get_role_member_count(&e, &USER_ROLE);
        assert_eq!(count_after, 1);

        // Only account2 should remain and should be at index 0 (the swap happened)
        let retrieved = get_role_member(&e, &USER_ROLE, 0);
        assert_eq!(retrieved, account2);

        // account1 should no longer have the role
        let has_role1 = has_role(&e, &account1, &USER_ROLE);
        assert_eq!(has_role1, None);

        // account2 should still have the role
        let has_role2 = has_role(&e, &account2, &USER_ROLE);
        assert_eq!(has_role2, Some(0));
    });
}

#[test]
fn remove_from_role_enumeration_for_last_account_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        // Add one account
        add_to_role_enumeration(&e, &account, &USER_ROLE);

        // Initial count should be 1
        let count_before = get_role_member_count(&e, &USER_ROLE);
        assert_eq!(count_before, 1);

        // Remove the account
        remove_from_role_enumeration(&e, &account, &USER_ROLE);

        // Count should be 0
        let count_after = get_role_member_count(&e, &USER_ROLE);
        assert_eq!(count_after, 0);

        // Account should no longer have the role
        let has_role = has_role(&e, &account, &USER_ROLE);
        assert_eq!(has_role, None);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #122)")]
fn remove_from_role_enumeration_with_nonexistent_role_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    let nonexistent_role = Symbol::new(&e, "nonexistent");

    e.as_contract(&address, || {
        // Attempt to remove account from a role that doesn't exist
        remove_from_role_enumeration(&e, &account, &nonexistent_role);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #122)")]
fn remove_from_role_enumeration_with_account_not_in_role_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account1 = Address::generate(&e);
    let account2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add one account to the role
        add_to_role_enumeration(&e, &account1, &USER_ROLE);

        // Attempt to remove a different account that doesn't have the role
        remove_from_role_enumeration(&e, &account2, &USER_ROLE);
    });
}
