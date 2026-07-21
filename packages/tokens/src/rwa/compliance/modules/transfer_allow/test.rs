extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events as _},
    vec, Address, Env,
};

use crate::rwa::compliance::{
    modules::transfer_allow::storage::{
        allow_user, batch_allow_users, batch_disallow_users, disallow_user, is_user_allowed,
        on_transfer, remove_user_allowed, set_user_allowed,
    },
    TransferKind,
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
fn on_transfer_allows_allowlisted_sender() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &from);

        on_transfer(&e, &from, &to, &TransferKind::Standard, &token);
    });
}

#[test]
fn on_transfer_allows_allowlisted_recipient() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &to);

        on_transfer(&e, &from, &to, &TransferKind::Standard, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #406)")]
fn on_transfer_panics_when_neither_party_allowlisted() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_transfer(&e, &from, &to, &TransferKind::Standard, &token);
    });
}

#[test]
fn on_transfer_forced_is_exempt_from_policy() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // Neither party is allowlisted: a standard transfer would panic,
        // but a forced one passes through untouched.
        on_transfer(&e, &from, &to, &TransferKind::Forced, &token);
    });
}

#[test]
fn on_transfer_recovery_is_exempt_from_policy() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // Neither party is allowlisted: a standard transfer would panic,
        // but a recovery passes through untouched.
        on_transfer(&e, &from, &to, &TransferKind::Recovery, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #406)")]
fn allowlist_is_tracked_per_token() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token_a, &from);

        // Allowed for token_a...
        on_transfer(&e, &from, &to, &TransferKind::Standard, &token_a);
        // ...but the allowlist does not carry over to token_b.
        on_transfer(&e, &from, &to, &TransferKind::Standard, &token_b);
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
#[should_panic(expected = "Error(Contract, #406)")]
fn disallow_user_revokes_access() {
    let e = Env::default();
    let module_id = e.register(TestTransferAllowContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        allow_user(&e, &token, &from);
        on_transfer(&e, &from, &to, &TransferKind::Standard, &token);

        disallow_user(&e, &token, &from);
        on_transfer(&e, &from, &to, &TransferKind::Standard, &token);
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
