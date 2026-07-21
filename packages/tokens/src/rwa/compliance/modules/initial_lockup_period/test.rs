extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, Vec,
};

use crate::rwa::compliance::{
    modules::initial_lockup_period::storage::{
        debit_forced, get_locked_amount, get_locked_details, get_lockup_period,
        is_preset_completed, mark_preset_completed, migrate_locks, on_created, on_destroyed,
        on_transfer, preset_locks, set_lockup_period, LockedTokens,
    },
    TransferKind,
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
        assert_eq!(get_locked_amount(&e, &token, &wallet), 80);
    });
}

#[test]
fn on_created_without_period_creates_no_lock() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_created(&e, &wallet, 80, &token);

        let details = get_locked_details(&e, &token, &wallet);
        assert_eq!(details.total_locked, 0);
        assert_eq!(details.locks.len(), 0);
        assert_eq!(get_locked_amount(&e, &token, &wallet), 0);
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
        assert_eq!(get_locked_amount(&e, &token, &wallet), 0);
    });
}

#[test]
fn on_transfer_allows_free_portion_only() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // 100 of the wallet's 150 stay locked until t=1000.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        // Spending the free portion succeeds and leaves the lock untouched.
        on_transfer(&e, &from, &to, 150, 50, &TransferKind::Standard, &token);
        assert_eq!(get_locked_details(&e, &token, &from).total_locked, 100);
    });
}

#[test]
fn on_transfer_without_locks_succeeds() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // No lockup period configured, so the mint creates no lock.
        on_created(&e, &from, 80, &token);

        on_transfer(&e, &from, &to, 80, 30, &TransferKind::Standard, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 0);
        assert_eq!(details.locks.len(), 0);
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
        // Sender holds 100 going into the transfer.
        on_transfer(&e, &from, &to, 100, 60, &TransferKind::Standard, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 40);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 40, release_ledger: 100 }]);
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
        // Sender holds 100 going into the transfer.
        on_transfer(&e, &from, &to, 100, 70, &TransferKind::Standard, &token);

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
        on_transfer(&e, &from, &to, 100, 50, &TransferKind::Standard, &token);

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

        on_transfer(&e, &from, &to, 80, 1, &TransferKind::Standard, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn on_transfer_delegated_cannot_spend_locked_tokens() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&module_id, || {
        // The wallet's entire balance stays locked until t=1000.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 80, release_ledger: 1_000 }],
        );

        // A delegated transfer (approved spender) is not privileged: dipping
        // into the locked region is rejected exactly like a standard one.
        on_transfer(&e, &from, &to, 80, 1, &TransferKind::Delegated(spender), &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn on_transfer_panics_when_balance_insufficient() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // No locks, but the spend exceeds the wallet's balance.
        on_transfer(&e, &from, &to, 5, 10, &TransferKind::Standard, &token);
    });
}

#[test]
fn on_transfer_forced_consumes_active_locks() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);

        // Everything is still locked, but a forced transfer (seizure) is not
        // rejected: the moved tokens' locks are consumed.
        on_transfer(&e, &from, &to, 80, 80, &TransferKind::Forced, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 0);
        assert_eq!(details.locks.len(), 0);
    });
}

#[test]
fn on_transfer_forced_uses_free_portion_first() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // 100 of the wallet's 150 stay locked until t=1000.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        // Forcing 70 out: 50 covered by the free portion, 20 consumed from
        // the active lock.
        on_transfer(&e, &from, &to, 150, 70, &TransferKind::Forced, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 80);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 80, release_ledger: 1_000 }]);
    });
}

#[test]
fn debit_forced_consumes_oldest_first_across_expiry() {
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

        // Neither lock has released; forced debit still consumes
        // oldest-first.
        on_transfer(&e, &from, &to, 100, 70, &TransferKind::Forced, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 30);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 30, release_ledger: 110 }]);
    });
}

#[test]
fn debit_forced_within_free_portion_leaves_locks_untouched() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // A wallet with no locks: the forced debit has nothing to consume.
        on_transfer(&e, &from, &to, 80, 30, &TransferKind::Forced, &token);
        assert_eq!(get_locked_details(&e, &token, &from).locks.len(), 0);

        // 100 of the wallet's 150 stay locked until t=1000; forcing out the
        // free 50 leaves the lock schedule untouched.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );
        on_transfer(&e, &from, &to, 150, 50, &TransferKind::Forced, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 100);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }]);
    });
}

#[test]
fn debit_forced_carries_over_locks_past_the_consumed_amount() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // The wallet's full balance of 100 is locked across two entries.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![
                &e,
                LockedTokens { amount: 40, release_ledger: 500 },
                LockedTokens { amount: 60, release_ledger: 1_000 },
            ],
        );

        // Forcing out exactly the first lock's amount: consumption ends at
        // the lock boundary and the second entry is carried over untouched.
        on_transfer(&e, &from, &to, 100, 40, &TransferKind::Forced, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 60);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 60, release_ledger: 1_000 }]);
    });
}

#[test]
fn debit_forced_tolerates_lock_shortfall() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);

    e.as_contract(&module_id, || {
        // Locks exceed the wallet's actual balance (a wrongly seeded preset):
        // the forced debit consumes what exists and never panics.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        debit_forced(&e, &token, &from, 50, 50);
        assert_eq!(get_locked_details(&e, &token, &from).total_locked, 50);
    });
}

#[test]
fn on_transfer_recovery_migrates_active_locks() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);

        // Everything is still locked; a recovery is not rejected and the
        // schedule follows the balance onto the new wallet.
        on_transfer(&e, &from, &to, 80, 80, &TransferKind::Recovery, &token);

        let source = get_locked_details(&e, &token, &from);
        assert_eq!(source.total_locked, 0);
        assert_eq!(source.locks.len(), 0);

        let destination = get_locked_details(&e, &token, &to);
        assert_eq!(destination.total_locked, 80);
        assert_eq!(destination.locks, vec![&e, LockedTokens { amount: 80, release_ledger: 100 }]);
    });
}

#[test]
fn on_transfer_recovery_migrates_only_the_locked_portion() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // 100 of the wallet's 150 stay locked until t=1000; recovering the
        // whole balance moves the lock, and only the lock, to the new wallet.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        on_transfer(&e, &from, &to, 150, 150, &TransferKind::Recovery, &token);

        assert_eq!(get_locked_details(&e, &token, &from).total_locked, 0);

        let destination = get_locked_details(&e, &token, &to);
        assert_eq!(destination.total_locked, 100);
        assert_eq!(
            destination.locks,
            vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }]
        );
    });
}

#[test]
fn on_transfer_recovery_preserves_release_ledgers() {
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

        // Both entries migrate intact, keeping their distinct release times.
        on_transfer(&e, &from, &to, 100, 100, &TransferKind::Recovery, &token);

        let destination = get_locked_details(&e, &token, &to);
        assert_eq!(destination.total_locked, 100);
        assert_eq!(
            destination.locks,
            vec![
                &e,
                LockedTokens { amount: 50, release_ledger: 100 },
                LockedTokens { amount: 50, release_ledger: 110 },
            ]
        );
    });
}

#[test]
fn on_transfer_recovery_partial_amount_splits_lock() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // 100 of the wallet's 150 stay locked until t=1000. Recovering 70:
        // 50 covered by the free portion, 20 split out of the active lock.
        // The consumed piece moves; the remainder stays; both keep the
        // release time.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        on_transfer(&e, &from, &to, 150, 70, &TransferKind::Recovery, &token);

        let source = get_locked_details(&e, &token, &from);
        assert_eq!(source.total_locked, 80);
        assert_eq!(source.locks, vec![&e, LockedTokens { amount: 80, release_ledger: 1_000 }]);

        let destination = get_locked_details(&e, &token, &to);
        assert_eq!(destination.total_locked, 20);
        assert_eq!(destination.locks, vec![&e, LockedTokens { amount: 20, release_ledger: 1_000 }]);
    });
}

#[test]
fn on_transfer_recovery_within_free_portion_leaves_locks_untouched() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // 100 of the wallet's 150 stay locked until t=1000. A partial
        // recovery covered entirely by the free portion dips into no lock:
        // nothing migrates and both schedules stay untouched.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        on_transfer(&e, &from, &to, 150, 50, &TransferKind::Recovery, &token);

        let source = get_locked_details(&e, &token, &from);
        assert_eq!(source.total_locked, 100);
        assert_eq!(source.locks, vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }]);
        assert_eq!(get_locked_details(&e, &token, &to).locks.len(), 0);
    });
}

#[test]
fn on_transfer_recovery_merges_with_destination_locks() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // The destination already carries a lock of its own; the migrated
        // entries are appended and the aggregates add up.
        preset_locks(&e, &token, &to, &vec![&e, LockedTokens { amount: 30, release_ledger: 500 }]);
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        on_transfer(&e, &from, &to, 100, 100, &TransferKind::Recovery, &token);

        let destination = get_locked_details(&e, &token, &to);
        assert_eq!(destination.total_locked, 130);
        assert_eq!(
            destination.locks,
            vec![
                &e,
                LockedTokens { amount: 30, release_ledger: 500 },
                LockedTokens { amount: 100, release_ledger: 1_000 },
            ]
        );
    });
}

#[test]
fn on_transfer_recovery_without_locks_touches_nothing() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // No lockup period configured, so the mint creates no lock.
        on_created(&e, &from, 80, &token);

        on_transfer(&e, &from, &to, 80, 80, &TransferKind::Recovery, &token);

        assert_eq!(get_locked_details(&e, &token, &from).locks.len(), 0);
        assert_eq!(get_locked_details(&e, &token, &to).locks.len(), 0);
    });
}

#[test]
fn on_transfer_recovery_to_same_wallet_keeps_schedule() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);

        // A recovery onto the same wallet reduces to a no-op instead of
        // duplicating the entries.
        on_transfer(&e, &from, &from, 80, 80, &TransferKind::Recovery, &token);

        let details = get_locked_details(&e, &token, &from);
        assert_eq!(details.total_locked, 80);
        assert_eq!(details.locks, vec![&e, LockedTokens { amount: 80, release_ledger: 100 }]);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn migrated_locks_still_enforced_on_destination() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_lockup_period(&e, &token, 100);
        on_created(&e, &from, 80, &token);
        on_transfer(&e, &from, &to, 80, 80, &TransferKind::Recovery, &token);

        // The migrated schedule keeps restricting the recovered balance:
        // spending from the new wallet before release is rejected.
        on_transfer(&e, &to, &from, 80, 1, &TransferKind::Standard, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #391)")]
fn migrate_locks_panics_when_destination_aggregate_overflows() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // The destination's lock aggregate already sits at the i128 ceiling,
        // so crediting the migrated entry overflows.
        preset_locks(
            &e,
            &token,
            &to,
            &vec![&e, LockedTokens { amount: i128::MAX, release_ledger: 1_000 }],
        );
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        on_transfer(&e, &from, &to, 100, 100, &TransferKind::Recovery, &token);
    });
}

#[test]
fn migrate_locks_tolerates_lock_shortfall() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // Locks exceed the wallet's actual balance (a wrongly seeded preset):
        // the migration moves what the balance covers and never panics.
        preset_locks(
            &e,
            &token,
            &from,
            &vec![&e, LockedTokens { amount: 100, release_ledger: 1_000 }],
        );

        migrate_locks(&e, &token, &from, &to, 50, 50);

        assert_eq!(get_locked_details(&e, &token, &from).total_locked, 50);
        assert_eq!(get_locked_details(&e, &token, &to).total_locked, 50);
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
        on_destroyed(&e, &wallet, 100, 60, &token);

        assert_eq!(get_locked_details(&e, &token, &wallet).total_locked, 40);
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

        on_destroyed(&e, &wallet, 80, 1, &token);
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
fn get_locked_amount_excludes_expired() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_locks(
            &e,
            &token,
            &wallet,
            &vec![
                &e,
                LockedTokens { amount: 30, release_ledger: 50 },
                LockedTokens { amount: 20, release_ledger: 1_000 },
            ],
        );

        // Nothing has expired yet: both locks still count.
        assert_eq!(get_locked_amount(&e, &token, &wallet), 50);

        // The 30-token lock has expired; only the 20-token lock remains.
        e.ledger().with_mut(|li| li.sequence_number = 60);
        assert_eq!(get_locked_amount(&e, &token, &wallet), 20);
    });
}

#[test]
fn preset_locks_writes_state() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        let locks = vec![&e, LockedTokens { amount: 40, release_ledger: 500 }];
        preset_locks(&e, &token, &wallet, &locks);

        let details = get_locked_details(&e, &token, &wallet);
        assert_eq!(details.total_locked, 40);
        assert_eq!(details.locks, locks);
        assert!(!is_preset_completed(&e, &token));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn preset_locks_panics_on_negative_lock_amount() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_locks(
            &e,
            &token,
            &wallet,
            &vec![&e, LockedTokens { amount: -1, release_ledger: 500 }],
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #395)")]
fn preset_locks_panics_after_preset_completed() {
    let e = Env::default();
    let module_id = e.register(TestInitialLockupPeriodContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        mark_preset_completed(&e, &token);
        preset_locks(&e, &token, &wallet, &Vec::new(&e));
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
