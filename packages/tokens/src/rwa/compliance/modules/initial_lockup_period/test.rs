extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, Vec,
};

use crate::rwa::compliance::modules::initial_lockup_period::storage::{
    can_transfer, get_locked_details, get_lockup_period, get_tracked_balance, get_unlocked_balance,
    is_preset_completed, mark_preset_completed, on_created, on_destroyed, on_transfer,
    preset_lockup_state, set_lockup_period, LockedTokens,
};

#[contract]
struct TestInitialLockupPeriodContract;

#[test]
fn set_lockup_period_persists_value() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert_eq!(get_lockup_period(&e, &token), 0);

        set_lockup_period(&e, &token, 17_280);
        assert_eq!(get_lockup_period(&e, &token), 17_280);
    });
}

#[test]
fn on_created_locks_minted_tokens() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &wallet, 80, &token);

        let details = get_locked_details(&e, &token, &wallet);
        assert_eq!(details.total_locked, 80);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 80, release_ledger: 100 }]);
        assert_eq!(get_tracked_balance(&e, &token, &wallet), 80);
        assert_eq!(get_unlocked_balance(&e, &token, &wallet), 0);
    });
}

#[test]
fn on_created_without_period_tracks_balance_only() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_created(&e, &wallet, 80, &token);

        let details = get_locked_details(&e, &token, &wallet);
        assert_eq!(details.total_locked, 0);
        assert_eq!(details.locks.len(), 0);
        assert_eq!(get_tracked_balance(&e, &token, &wallet), 80);
        assert_eq!(get_unlocked_balance(&e, &token, &wallet), 80);
    });
}

#[test]
fn on_created_zero_amount_creates_no_lock() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &wallet, 0, &token);

        assert_eq!(get_locked_details(&e, &token, &wallet).locks.len(), 0);
        assert_eq!(get_tracked_balance(&e, &token, &wallet), 0);
    });
}

#[test]
fn can_transfer_rejects_locked_tokens() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);

        assert!(!can_transfer(&e, &from, &to, 1, &token));
        assert!(can_transfer(&e, &from, &to, 0, &token));
    });
}

#[test]
fn can_transfer_allows_free_portion_only() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // 150 mirrored, of which 100 stay locked until t=1000.
        preset_lockup_state(
            &e,
            &token,
            &from,
            150,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        assert!(can_transfer(&e, &from, &to, 50, &token));
        assert!(!can_transfer(&e, &from, &to, 51, &token));
    });
}

#[test]
fn can_transfer_allows_after_release() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);
        assert!(!can_transfer(&e, &from, &to, 80, &token));

        e.ledger().with_mut(|li| li.sequence_number = 100);
        assert!(can_transfer(&e, &from, &to, 80, &token));
    });
}

#[test]
fn can_transfer_rejects_unseeded_wallet() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // The balance mirror is authoritative: a wallet that was never
        // credited (or seeded via preset) has nothing to spend.
        assert!(!can_transfer(&e, &from, &to, 1, &token));
    });
}

#[test]
fn on_transfer_moves_balance_between_wallets() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_created(&e, &from, 80, &token);

        on_transfer(&e, &from, &to, 30, &token);

        assert_eq!(get_tracked_balance(&e, &token, &from), 50);
        assert_eq!(get_tracked_balance(&e, &token, &to), 30);
        // Tokens received through transfers are never locked.
        assert_eq!(get_locked_details(&e, &token, &to).total_locked, 0);
    });
}

#[test]
fn on_transfer_consumes_expired_locks() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 100, &token);

        e.ledger().with_mut(|li| li.sequence_number = 100);
        on_transfer(&e, &from, &to, 60, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 40);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 40, release_ledger: 100 }]);
        assert_eq!(get_tracked_balance(&e, &token, &from), 40);
        assert_eq!(get_tracked_balance(&e, &token, &to), 60);
    });
}

#[test]
fn on_transfer_consumes_locks_oldest_first() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 50, &token); // releases at t=100
        e.ledger().with_mut(|li| li.sequence_number = 10);
        on_created(&e, &from, 50, &token); // releases at t=110

        e.ledger().with_mut(|li| li.sequence_number = 200);
        on_transfer(&e, &from, &to, 70, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 30);
        // The first lock is fully consumed; the second keeps its remainder.
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 30, release_ledger: 110 }]);
    });
}

#[test]
fn on_transfer_keeps_unexpired_locks_intact() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 50, &token); // releases at t=100
        e.ledger().with_mut(|li| li.sequence_number = 50);
        on_created(&e, &from, 50, &token); // releases at t=150

        // Only the first lock has expired.
        e.ledger().with_mut(|li| li.sequence_number = 120);
        on_transfer(&e, &from, &to, 50, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 50);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 50, release_ledger: 150 }]);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn on_transfer_panics_when_tokens_still_locked() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);

        on_transfer(&e, &from, &to, 1, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #392)")]
fn on_transfer_panics_when_mirror_balance_insufficient() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_created(&e, &from, 5, &token);

        on_transfer(&e, &from, &to, 10, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn on_created_panics_on_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_created(&e, &wallet, -1, &token);
    });
}

#[test]
fn on_destroyed_consumes_expired_locks() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &wallet, 100, &token);

        e.ledger().with_mut(|li| li.sequence_number = 100);
        on_destroyed(&e, &wallet, 60, &token);

        assert_eq!(get_locked_details(&e, &token, &wallet).total_locked, 40);
        assert_eq!(get_tracked_balance(&e, &token, &wallet), 40);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn on_destroyed_panics_when_tokens_still_locked() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &wallet, 80, &token);

        on_destroyed(&e, &wallet, 1, &token);
    });
}

#[test]
fn get_unlocked_balance_combines_free_and_expired() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_lockup_state(
            &e,
            &token,
            &wallet,
            100,
            &vec![
                &e,
                LockedTokens { amount: 30, release_ledger: 50 },
                LockedTokens { amount: 20, release_ledger: 1_000 },
            ],
        );

        // Free portion only: 100 - 50 locked.
        assert_eq!(get_unlocked_balance(&e, &token, &wallet), 50);

        // The 30-token lock has expired: free (50) + expired (30).
        e.ledger().with_mut(|li| li.sequence_number = 60);
        assert_eq!(get_unlocked_balance(&e, &token, &wallet), 80);
    });
}

#[test]
fn preset_lockup_state_writes_state() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        let locks = vec![&e, LockedTokens { amount: 40, release_ledger: 500 }];
        preset_lockup_state(&e, &token, &wallet, 100, &locks);

        let details = get_locked_details(&e, &token, &wallet);
        assert_eq!(details.total_locked, 40);
        assert_eq!(details.locks, locks);
        assert_eq!(get_tracked_balance(&e, &token, &wallet), 100);
        assert!(!is_preset_completed(&e, &token));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #400)")]
fn preset_lockup_state_panics_when_locked_exceeds_balance() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_lockup_state(
            &e,
            &token,
            &wallet,
            100,
            &vec![&e, LockedTokens { amount: 101, release_ledger: 500 }],
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn preset_lockup_state_panics_on_negative_lock_amount() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_lockup_state(
            &e,
            &token,
            &wallet,
            100,
            &vec![&e, LockedTokens { amount: -1, release_ledger: 500 }],
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #395)")]
fn preset_lockup_state_panics_after_preset_completed() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        mark_preset_completed(&e, &token);
        preset_lockup_state(&e, &token, &wallet, 100, &Vec::new(&e));
    });
}

#[test]
fn mark_preset_completed_flips_flag() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert!(!is_preset_completed(&e, &token));

        mark_preset_completed(&e, &token);
        assert!(is_preset_completed(&e, &token));
    });
}
