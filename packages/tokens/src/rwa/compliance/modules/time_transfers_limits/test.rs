extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Events as _, Ledger as _},
    vec, Address, Env, Val, Vec,
};

use crate::rwa::{
    compliance::modules::{
        storage::set_irs_address,
        time_transfers_limits::{
            storage::{
                batch_remove_time_transfer_limit, batch_set_time_transfer_limit, can_transfer,
                get_time_transfer_limits, get_transfer_counter, on_transfer,
                remove_time_transfer_limit, set_time_transfer_limit, TransferCounter,
                TransferLimit,
            },
            MAX_LIMITS,
        },
    },
    identity_registry_storage::{CountryDataManager, IdentityRegistryStorage},
    utils::token_binder::TokenBinder,
};

#[contract]
struct MockIRSContract;

#[contracttype]
#[derive(Clone)]
enum MockIRSStorageKey {
    Identity(Address),
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
        e.storage()
            .persistent()
            .get(&MockIRSStorageKey::Identity(account.clone()))
            .unwrap_or(account)
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
}

#[contract]
struct TestTimeTransfersLimitsContract;

#[test]
fn set_time_transfer_limit_adds_and_updates_entries() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 200, limit_value: 80 },
        );

        // Re-using a window duration replaces the existing entry in place.
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 60 },
        );

        assert_eq!(
            get_time_transfer_limits(&e, &token),
            vec![
                &e,
                TransferLimit { limit_duration: 100, limit_value: 60 },
                TransferLimit { limit_duration: 200, limit_value: 80 },
            ]
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn set_time_transfer_limit_panics_at_bound() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        for limit_duration in 1..=MAX_LIMITS {
            set_time_transfer_limit(&e, &token, &TransferLimit { limit_duration, limit_value: 50 });
        }

        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: MAX_LIMITS + 1, limit_value: 50 },
        );
    });
}

#[test]
fn set_time_transfer_limit_updates_existing_at_bound() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        for limit_duration in 1..=MAX_LIMITS {
            set_time_transfer_limit(&e, &token, &TransferLimit { limit_duration, limit_value: 50 });
        }

        // Updating an existing window does not count against the bound.
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: MAX_LIMITS, limit_value: 99 },
        );

        let limits = get_time_transfer_limits(&e, &token);
        assert_eq!(limits.len(), MAX_LIMITS);
        assert_eq!(
            limits.get_unchecked(MAX_LIMITS - 1),
            TransferLimit { limit_duration: MAX_LIMITS, limit_value: 99 }
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn set_time_transfer_limit_panics_on_negative_value() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: -1 },
        );
    });
}

#[test]
fn remove_time_transfer_limit_removes_entry() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 200, limit_value: 80 },
        );

        remove_time_transfer_limit(&e, &token, 100);

        assert_eq!(
            get_time_transfer_limits(&e, &token),
            vec![&e, TransferLimit { limit_duration: 200, limit_value: 80 }]
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #403)")]
fn remove_time_transfer_limit_panics_when_missing() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        remove_time_transfer_limit(&e, &token, 100);
    });
}

#[test]
fn batch_set_and_remove_time_transfer_limits() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        batch_set_time_transfer_limit(
            &e,
            &token,
            &vec![
                &e,
                TransferLimit { limit_duration: 100, limit_value: 50 },
                TransferLimit { limit_duration: 200, limit_value: 80 },
            ],
        );
        assert_eq!(get_time_transfer_limits(&e, &token).len(), 2);

        let events_before = e.events().all().events().len();
        batch_remove_time_transfer_limit(&e, &token, &vec![&e, 100_u32, 200_u32]);

        assert_eq!(get_time_transfer_limits(&e, &token).len(), 0);
        assert_eq!(e.events().all().events().len(), events_before + 2);
    });
}

#[test]
fn can_transfer_true_when_no_limits_configured() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // No IRS is configured either: the identity lookup must be skipped
        // entirely when no limits exist.
        assert!(can_transfer(&e, &from, &to, 100, &token));
    });
}

#[test]
fn can_transfer_rejects_amount_above_cap() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        assert!(can_transfer(&e, &from, &to, 50, &token));
        assert!(!can_transfer(&e, &from, &to, 51, &token));
    });
}

#[test]
fn can_transfer_accounts_for_running_counter() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        on_transfer(&e, &from, &to, 30, &token);

        assert!(can_transfer(&e, &from, &to, 20, &token));
        assert!(!can_transfer(&e, &from, &to, 21, &token));
    });
}

#[test]
fn can_transfer_allows_again_after_window_elapses() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        on_transfer(&e, &from, &to, 50, &token);
        assert!(!can_transfer(&e, &from, &to, 1, &token));

        e.ledger().with_mut(|li| li.sequence_number = 100);
        assert!(can_transfer(&e, &from, &to, 50, &token));
    });
}

#[test]
fn on_transfer_increments_counters_for_each_window() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 200, limit_value: 80 },
        );

        on_transfer(&e, &from, &to, 30, &token);

        // With no identity registered, the wallet acts as its own identity.
        assert_eq!(
            get_transfer_counter(&e, &token, &from, 100),
            TransferCounter { value: 30, deadline: 100 }
        );
        assert_eq!(
            get_transfer_counter(&e, &token, &from, 200),
            TransferCounter { value: 30, deadline: 200 }
        );
    });
}

#[test]
fn on_transfer_aggregates_volume_per_identity() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let wallet_a = Address::generate(&e);
    let wallet_b = Address::generate(&e);
    let identity = Address::generate(&e);
    let to = Address::generate(&e);

    // Two wallets, same identity.
    irs.set_identity(&wallet_a, &identity);
    irs.set_identity(&wallet_b, &identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        on_transfer(&e, &wallet_a, &to, 30, &token);

        // Splitting the volume across wallets does not raise the cap.
        assert_eq!(get_transfer_counter(&e, &token, &identity, 100).value, 30);
        assert!(can_transfer(&e, &wallet_b, &to, 20, &token));
        assert!(!can_transfer(&e, &wallet_b, &to, 21, &token));
    });
}

#[test]
fn on_transfer_restarts_elapsed_window() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        on_transfer(&e, &from, &to, 50, &token);

        e.ledger().with_mut(|li| li.sequence_number = 150);
        on_transfer(&e, &from, &to, 10, &token);

        // The elapsed counter restarted instead of accumulating.
        assert_eq!(
            get_transfer_counter(&e, &token, &from, 100),
            TransferCounter { value: 10, deadline: 250 }
        );
    });
}

#[test]
fn on_transfer_skips_when_no_limits_configured() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // No limits and no IRS: the hook is a no-op rather than a panic.
        on_transfer(&e, &from, &to, 100, &token);

        assert_eq!(get_transfer_counter(&e, &token, &from, 100).value, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #401)")]
fn on_transfer_panics_when_limit_exceeded() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_time_transfer_limit(
            &e,
            &token,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        on_transfer(&e, &from, &to, 30, &token);
        on_transfer(&e, &from, &to, 21, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn can_transfer_panics_on_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        can_transfer(&e, &from, &to, -1, &token);
    });
}

#[test]
fn counters_are_tracked_per_token() {
    let e = Env::default();
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token_a, &irs_id);
        set_irs_address(&e, &token_b, &irs_id);
        set_time_transfer_limit(
            &e,
            &token_a,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );
        set_time_transfer_limit(
            &e,
            &token_b,
            &TransferLimit { limit_duration: 100, limit_value: 50 },
        );

        on_transfer(&e, &from, &to, 50, &token_a);

        assert!(!can_transfer(&e, &from, &to, 1, &token_a));
        assert!(can_transfer(&e, &from, &to, 50, &token_b));
    });
}
