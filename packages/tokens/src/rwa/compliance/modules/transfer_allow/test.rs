extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events as _},
    vec, Address, Env,
};

use crate::rwa::compliance::modules::transfer_allow::storage::{
    allow_user, batch_allow_users, batch_disallow_users, can_transfer, disallow_user,
    is_user_allowed, remove_user_allowed, set_user_allowed,
};

#[contract]
struct TestTransferAllowContract;

#[test]
fn set_user_allowed_persists_membership() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert!(!is_user_allowed(&e, &token, &user));

        set_user_allowed(&e, &token, &user);
        assert!(is_user_allowed(&e, &token, &user));

        remove_user_allowed(&e, &token, &user);
        assert!(!is_user_allowed(&e, &token, &user));
    });
}

#[test]
fn can_transfer_allows_allowlisted_sender() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &from);

        assert!(can_transfer(&e, &from, &to, 100, &token));
    });
}

#[test]
fn can_transfer_allows_allowlisted_recipient() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &to);

        assert!(can_transfer(&e, &from, &to, 100, &token));
    });
}

#[test]
fn can_transfer_rejects_when_neither_party_allowlisted() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert!(!can_transfer(&e, &from, &to, 100, &token));
    });
}

#[test]
fn allowlist_is_tracked_per_token() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token_a, &from);

        assert!(can_transfer(&e, &from, &to, 100, &token_a));
        assert!(!can_transfer(&e, &from, &to, 100, &token_b));
    });
}

#[test]
fn allow_user_is_idempotent() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &user);
        let after_first = e.events().all().events().len();

        allow_user(&e, &token, &user);

        assert!(is_user_allowed(&e, &token, &user));
        assert_eq!(e.events().all().events().len(), after_first);
    });
}

#[test]
fn disallow_user_is_noop_when_not_present() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&module_id, || {
        let before = e.events().all().events().len();

        disallow_user(&e, &token, &user);

        assert!(!is_user_allowed(&e, &token, &user));
        assert_eq!(e.events().all().events().len(), before);
    });
}

#[test]
fn disallow_user_revokes_access() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &from);
        assert!(can_transfer(&e, &from, &to, 100, &token));

        disallow_user(&e, &token, &from);
        assert!(!can_transfer(&e, &from, &to, 100, &token));
    });
}

#[test]
fn batch_allow_users_adds_all_entries() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let user_a = Address::generate(&e);
    let user_b = Address::generate(&e);

    e.as_contract(&module_id, || {
        batch_allow_users(&e, &token, &vec![&e, user_a.clone(), user_b.clone()]);

        assert!(is_user_allowed(&e, &token, &user_a));
        assert!(is_user_allowed(&e, &token, &user_b));
    });
}

#[test]
fn batch_disallow_users_removes_all_entries() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let user_a = Address::generate(&e);
    let user_b = Address::generate(&e);

    e.as_contract(&module_id, || {
        batch_allow_users(&e, &token, &vec![&e, user_a.clone(), user_b.clone()]);

        batch_disallow_users(&e, &token, &vec![&e, user_a.clone(), user_b.clone()]);

        assert!(!is_user_allowed(&e, &token, &user_a));
        assert!(!is_user_allowed(&e, &token, &user_b));
    });
}
