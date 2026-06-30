extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error,
    testutils::{Address as _, Events as _},
    vec, Address, Env, Val, Vec,
};

use crate::rwa::{
    compliance::{
        modules::{
            max_balance::storage::{
                batch_preset_id_balances, get_id_balance, get_id_balance_of, get_max_balance,
                is_preset_completed, mark_preset_completed, on_created, on_destroyed, on_transfer,
                preset_id_balance, set_max_balance,
            },
            storage::set_irs_address,
        },
        TransferKind,
    },
    identity_registry_storage::{CountryDataManager, IRSError, IdentityRegistryStorage},
    utils::token_binder::TokenBinder,
};

#[contract]
struct MockIRSContract;

#[contracttype]
#[derive(Clone)]
enum MockIRSStorageKey {
    Identity(Address),
    RecoveredTo(Address),
    Removed(Address),
}

#[contractimpl]
impl TokenBinder for MockIRSContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        Vec::new(e)
    }

    fn bind_token(_e: &Env, _token: Address, _operator: Address) {
        unreachable!("bind_token is not used in these tests");
    }

    fn unbind_token(_e: &Env, _token: Address, _operator: Address) {
        unreachable!("unbind_token is not used in these tests");
    }
}

#[contractimpl]
impl IdentityRegistryStorage for MockIRSContract {
    fn add_identity(
        _e: &Env,
        _account: Address,
        _identity: Address,
        _country_data_list: Vec<Val>,
        _operator: Address,
    ) {
        unreachable!("add_identity is not used in these tests");
    }

    fn remove_identity(_e: &Env, _account: Address, _operator: Address) {
        unreachable!("remove_identity is not used in these tests");
    }

    fn modify_identity(_e: &Env, _account: Address, _identity: Address, _operator: Address) {
        unreachable!("modify_identity is not used in these tests");
    }

    fn recover_identity(
        _e: &Env,
        _old_account: Address,
        _new_account: Address,
        _operator: Address,
    ) {
        unreachable!("recover_identity is not used in these tests");
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        if let Some(identity) = e
            .storage()
            .persistent()
            .get::<_, Address>(&MockIRSStorageKey::Identity(account.clone()))
        {
            identity
        } else if e.storage().persistent().has(&MockIRSStorageKey::RecoveredTo(account.clone()))
            || e.storage().persistent().has(&MockIRSStorageKey::Removed(account.clone()))
        {
            // Model the real IRS: once the mapping is gone (recovery or
            // `remove_identity`), the lookup reverts with `IdentityNotFound`.
            panic_with_error!(e, IRSError::IdentityNotFound)
        } else {
            account
        }
    }

    fn get_recovered_to(e: &Env, old_account: Address) -> Option<Address> {
        e.storage().persistent().get(&MockIRSStorageKey::RecoveredTo(old_account))
    }
}

#[contractimpl]
impl CountryDataManager for MockIRSContract {
    fn add_country_data_entries(
        _e: &Env,
        _account: Address,
        _country_data_list: Vec<Val>,
        _operator: Address,
    ) {
        unreachable!("add_country_data_entries is not used in these tests");
    }

    fn modify_country_data(
        _e: &Env,
        _account: Address,
        _index: u32,
        _country_data: Val,
        _operator: Address,
    ) {
        unreachable!("modify_country_data is not used in these tests");
    }

    fn delete_country_data(_e: &Env, _account: Address, _index: u32, _operator: Address) {
        unreachable!("delete_country_data is not used in these tests");
    }

    fn get_country_data_entries(_e: &Env, _account: Address) -> Vec<Val> {
        unreachable!("get_country_data_entries is not used in these tests");
    }
}

#[contractimpl]
impl MockIRSContract {
    pub fn set_identity(e: &Env, account: Address, identity: Address) {
        e.storage().persistent().set(&MockIRSStorageKey::Identity(account), &identity);
    }

    /// Models `recover_identity`: moves the identity from `old_account` to
    /// `new_account`, removes the old mapping, and records the recovery target.
    pub fn recover(e: &Env, old_account: Address, new_account: Address) {
        let old_key = MockIRSStorageKey::Identity(old_account.clone());
        let identity: Address =
            e.storage().persistent().get(&old_key).expect("identity must be set before recovery");
        e.storage().persistent().set(&MockIRSStorageKey::Identity(new_account.clone()), &identity);
        e.storage().persistent().remove(&old_key);
        e.storage().persistent().set(&MockIRSStorageKey::RecoveredTo(old_account), &new_account);
    }

    /// Models `remove_identity`: deletes the mapping without recording a
    /// recovery target, so the lookup reverts like the real IRS.
    pub fn remove(e: &Env, account: Address) {
        e.storage().persistent().remove(&MockIRSStorageKey::Identity(account.clone()));
        e.storage().persistent().set(&MockIRSStorageKey::Removed(account), &());
    }
}

#[contract]
struct TestMaxBalanceContract;

#[test]
fn set_max_balance_persists_value() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_max_balance(&e, &token, 1_000);
        assert_eq!(get_max_balance(&e, &token), 1_000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #393)")]
fn cap_applies_to_aggregate_balance_per_identity() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let wallet_a = Address::generate(&e);
    let wallet_b = Address::generate(&e);
    let identity = Address::generate(&e);

    // Two wallets, same identity.
    irs.set_identity(&wallet_a, &identity);
    irs.set_identity(&wallet_b, &identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);

        // Once wallet_a credits the identity, wallet_b is also capped:
        // crediting the remaining headroom succeeds, one token more panics.
        on_created(&e, &wallet_a, 70, &token);
        on_created(&e, &wallet_b, 30, &token);
        on_created(&e, &wallet_b, 1, &token);
    });
}

#[test]
fn on_transfer_moves_balance_between_identities() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let alice_wallet = Address::generate(&e);
    let bob_wallet = Address::generate(&e);
    let alice_id = Address::generate(&e);
    let bob_id = Address::generate(&e);

    irs.set_identity(&alice_wallet, &alice_id);
    irs.set_identity(&bob_wallet, &bob_id);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);
        on_created(&e, &alice_wallet, 80, &token);

        on_transfer(&e, &alice_wallet, &bob_wallet, 30, &TransferKind::Standard, &token);

        assert_eq!(get_id_balance(&e, &token, &alice_id), 50);
        assert_eq!(get_id_balance(&e, &token, &bob_id), 30);
    });
}

#[test]
fn same_identity_transfer_is_noop_for_cap_and_balance() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let wallet_a = Address::generate(&e);
    let wallet_b = Address::generate(&e);
    let identity = Address::generate(&e);

    irs.set_identity(&wallet_a, &identity);
    irs.set_identity(&wallet_b, &identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);
        on_created(&e, &wallet_a, 100, &token);

        // Identity is at the cap, but a transfer between two wallets of the
        // same identity must still be permitted.
        on_transfer(&e, &wallet_a, &wallet_b, 50, &TransferKind::Standard, &token);
        assert_eq!(get_id_balance(&e, &token, &identity), 100);
    });
}

#[test]
fn on_destroyed_decrements_identity_balance() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let identity = Address::generate(&e);

    irs.set_identity(&wallet, &identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);
        on_created(&e, &wallet, 80, &token);
        on_destroyed(&e, &wallet, 30, &token);

        assert_eq!(get_id_balance(&e, &token, &identity), 50);
        assert_eq!(get_id_balance_of(&e, &token, &wallet), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #393)")]
fn on_created_panics_when_exceeding_max() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 50);
        on_created(&e, &wallet, 51, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #392)")]
fn on_destroyed_panics_when_identity_has_insufficient_balance() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        on_destroyed(&e, &wallet, 1, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #393)")]
fn on_transfer_panics_when_recipient_exceeds_max() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let from_wallet = Address::generate(&e);
    let to_wallet = Address::generate(&e);
    let from_id = Address::generate(&e);
    let to_id = Address::generate(&e);

    irs.set_identity(&from_wallet, &from_id);
    irs.set_identity(&to_wallet, &to_id);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 50);

        // Pre-seed both identities so the debit half of the transfer
        // succeeds; we are testing the credit-side cap, not debit
        // underflow.
        preset_id_balance(&e, &token, &from_id, 50);
        preset_id_balance(&e, &token, &to_id, 30);

        // to_id is at 30/50; adding 25 puts the recipient at 55, past
        // the cap. from_id goes 50 -> 25, well within range.
        on_transfer(&e, &from_wallet, &to_wallet, 25, &TransferKind::Standard, &token);
    });
}

#[test]
fn on_transfer_forced_bypasses_cap_but_updates_books() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let from_wallet = Address::generate(&e);
    let to_wallet = Address::generate(&e);
    let from_id = Address::generate(&e);
    let to_id = Address::generate(&e);

    irs.set_identity(&from_wallet, &from_id);
    irs.set_identity(&to_wallet, &to_id);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 50);

        preset_id_balance(&e, &token, &from_id, 50);
        preset_id_balance(&e, &token, &to_id, 30);

        // The same movement that panics as a standard transfer goes through
        // when forced, and the recipient's aggregate records the over-cap
        // figure so the books stay true.
        on_transfer(&e, &from_wallet, &to_wallet, 25, &TransferKind::Forced, &token);

        assert_eq!(get_id_balance(&e, &token, &from_id), 25);
        assert_eq!(get_id_balance(&e, &token, &to_id), 55);
    });
}

// Regression: after `recover_identity` removes the old wallet's identity, the
// RWA recovery flow moves the balance via a forced transfer to the recovery
// target. The hook must not revert on the now-missing source identity; it
// resolves `from` through the recovery record, finds the same identity on both
// sides, and leaves the per-identity book untouched (a same-identity no-op).
#[test]
fn forced_transfer_after_recovery_is_noop_and_does_not_revert() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let old_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let identity = Address::generate(&e);

    irs.set_identity(&old_wallet, &identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);
        on_created(&e, &old_wallet, 80, &token);
    });

    // Identity recovery moves the identity to the new wallet and removes the
    // old mapping, so `stored_identity(old_wallet)` now reverts.
    irs.recover(&old_wallet, &new_wallet);

    e.as_contract(&module_id, || {
        // The documented recovery move: forced transfer of the whole balance
        // from the (now identity-less) old wallet to the recovery target.
        on_transfer(&e, &old_wallet, &new_wallet, 80, &TransferKind::Forced, &token);

        // Same identity on both sides => the aggregate is unchanged and correct.
        assert_eq!(get_id_balance(&e, &token, &identity), 80);
    });
}

// Regression: a forced transfer out of a recovered (identity-less) wallet to a
// *different* identity must still keep the books straight. The source resolves
// through the recovery record to its identity, which differs from the
// recipient's, so the move debits and credits both aggregates as normal.
#[test]
fn forced_transfer_after_recovery_to_third_party_updates_books() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let old_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let third_wallet = Address::generate(&e);
    let identity = Address::generate(&e);
    let third_id = Address::generate(&e);

    irs.set_identity(&old_wallet, &identity);
    irs.set_identity(&third_wallet, &third_id);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);
        on_created(&e, &old_wallet, 80, &token);
    });

    irs.recover(&old_wallet, &new_wallet);

    e.as_contract(&module_id, || {
        on_transfer(&e, &old_wallet, &third_wallet, 30, &TransferKind::Forced, &token);

        // `identity` (resolved via the recovery record) is debited; `third_id`
        // is credited unchecked because the transfer is forced.
        assert_eq!(get_id_balance(&e, &token, &identity), 50);
        assert_eq!(get_id_balance(&e, &token, &third_id), 30);
    });
}

// Regression: a sender whose identity was removed *without* recovery has no
// record to fall back to, so the hook re-raises the IRS error it captured
// (`IdentityNotFound`, #321) rather than silently swallowing it.
#[test]
#[should_panic(expected = "Error(Contract, #321)")]
fn on_transfer_reraises_irs_error_when_source_unregistered() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let from_wallet = Address::generate(&e);
    let to_wallet = Address::generate(&e);
    let from_id = Address::generate(&e);
    let to_id = Address::generate(&e);

    irs.set_identity(&from_wallet, &from_id);
    irs.set_identity(&to_wallet, &to_id);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_max_balance(&e, &token, 100);
        on_created(&e, &from_wallet, 50, &token);
    });

    // Identity removed without recovery: the source lookup reverts and there is
    // no recovery record, so the captured error must propagate unchanged.
    irs.remove(&from_wallet);

    e.as_contract(&module_id, || {
        on_transfer(&e, &from_wallet, &to_wallet, 10, &TransferKind::Standard, &token);
    });
}

#[test]
fn preset_id_balance_writes_directly() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let identity = Address::generate(&e);

    e.as_contract(&module_id, || {
        // Preset works even before set_max_balance is called: it is purely
        // a migration helper and does not enforce the cap.
        preset_id_balance(&e, &token, &identity, 9_999);

        assert_eq!(get_id_balance(&e, &token, &identity), 9_999);
        assert!(!is_preset_completed(&e, &token));
    });
}

#[test]
fn batch_preset_id_balances_writes_all_entries() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let id_a = Address::generate(&e);
    let id_b = Address::generate(&e);

    e.as_contract(&module_id, || {
        batch_preset_id_balances(
            &e,
            &token,
            &vec![&e, id_a.clone(), id_b.clone()],
            &vec![&e, 100_i128, 200_i128],
        );

        assert_eq!(get_id_balance(&e, &token, &id_a), 100);
        assert_eq!(get_id_balance(&e, &token, &id_b), 200);
    });
}

#[test]
fn mark_preset_completed_blocks_further_presets() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let identity = Address::generate(&e);

    e.as_contract(&module_id, || {
        mark_preset_completed(&e, &token);
        assert!(is_preset_completed(&e, &token));

        let after_mark = e.events().all().events().len();
        // Repeated marking is a write but emits another event; no panic.
        mark_preset_completed(&e, &token);
        assert_eq!(e.events().all().events().len(), after_mark + 1);

        // Preset attempts must panic — verified by the dedicated panic
        // test; here we only re-confirm the flag is sticky.
        assert!(is_preset_completed(&e, &token));
        let _ = identity;
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #395)")]
fn preset_id_balance_panics_after_preset_completed() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let identity = Address::generate(&e);

    e.as_contract(&module_id, || {
        mark_preset_completed(&e, &token);
        preset_id_balance(&e, &token, &identity, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #397)")]
fn batch_preset_id_balances_panics_on_length_mismatch() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let identity = Address::generate(&e);

    e.as_contract(&module_id, || {
        batch_preset_id_balances(&e, &token, &vec![&e, identity], &vec![&e, 1_i128, 2_i128]);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn preset_id_balance_panics_on_negative_balance() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let identity = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_id_balance(&e, &token, &identity, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn set_max_balance_panics_on_negative_value() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_max_balance(&e, &token, -1);
    });
}
