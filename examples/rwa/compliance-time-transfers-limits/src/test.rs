extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, String, Val, Vec,
};
use stellar_tokens::rwa::{
    compliance::{
        modules::time_transfers_limits::{TransferCounter, TransferLimit, MAX_LIMITS},
        AccountSnapshot,
    },
    identity_registry_storage::{CountryDataManager, IdentityRegistryStorage},
    utils::token_binder::TokenBinder,
};

use crate::contract::{TimeTransfersLimitsContract, TimeTransfersLimitsContractClient};

/// This module ignores balance and frozen amounts, so they are left at zero.
fn snap(address: &Address) -> AccountSnapshot {
    AccountSnapshot { address: address.clone(), balance: 0, frozen: 0 }
}

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> TimeTransfersLimitsContractClient<'a> {
    let address = e.register(TimeTransfersLimitsContract, (admin, manager));
    TimeTransfersLimitsContractClient::new(e, &address)
}

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

#[test]
fn set_and_get_time_transfer_limits_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.get_time_transfer_limits(&token).len(), 0);

    client.set_time_transfer_limit(
        &token,
        &TransferLimit { limit_duration: 100, limit_value: 50 },
        &manager,
    );
    // Re-using a window duration replaces the existing entry.
    client.set_time_transfer_limit(
        &token,
        &TransferLimit { limit_duration: 100, limit_value: 60 },
        &manager,
    );

    assert_eq!(
        client.get_time_transfer_limits(&token),
        vec![&e, TransferLimit { limit_duration: 100, limit_value: 60 }]
    );
}

#[test]
fn batch_set_and_remove_time_transfer_limits_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.batch_set_time_transfer_limit(
        &token,
        &vec![
            &e,
            TransferLimit { limit_duration: 100, limit_value: 50 },
            TransferLimit { limit_duration: 200, limit_value: 80 },
        ],
        &manager,
    );
    assert_eq!(client.get_time_transfer_limits(&token).len(), 2);

    client.batch_remove_time_transfer_limit(&token, &vec![&e, 100_u32, 200_u32], &manager);
    assert_eq!(client.get_time_transfer_limits(&token).len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #403)")]
fn remove_time_transfer_limit_panics_when_missing() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.remove_time_transfer_limit(&token, &100_u32, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn set_time_transfer_limit_panics_at_bound() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    for limit_duration in 1..=MAX_LIMITS + 1 {
        client.set_time_transfer_limit(
            &token,
            &TransferLimit { limit_duration, limit_value: 50 },
            &manager,
        );
    }
}

#[test]
fn transfers_accumulate_against_identity_windows() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet_a = Address::generate(&e);
    let wallet_b = Address::generate(&e);
    let identity = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);

    // Both wallets resolve to the same identity.
    irs.set_identity(&wallet_a, &identity);
    irs.set_identity(&wallet_b, &identity);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_identity_registry_storage(&token, &irs_id, &manager);
    client.set_time_transfer_limit(
        &token,
        &TransferLimit { limit_duration: 100, limit_value: 50 },
        &manager,
    );

    client.on_transfer(&snap(&wallet_a), &snap(&to), &30_i128, &None, &token);

    assert_eq!(
        client.get_transfer_counter(&token, &identity, &100_u32),
        TransferCounter { value: 30, deadline: 100 }
    );

    // Splitting the volume across wallets does not raise the cap.
    assert!(client.can_transfer(&snap(&wallet_b), &snap(&to), &20_i128, &None, &token));
    assert!(!client.can_transfer(&snap(&wallet_b), &snap(&to), &21_i128, &None, &token));
}

#[test]
fn window_elapses_and_counting_restarts() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let irs_id = e.register(MockIRSContract, ());

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_identity_registry_storage(&token, &irs_id, &manager);
    client.set_time_transfer_limit(
        &token,
        &TransferLimit { limit_duration: 100, limit_value: 50 },
        &manager,
    );

    client.on_transfer(&snap(&from), &snap(&to), &50_i128, &None, &token);
    assert!(!client.can_transfer(&snap(&from), &snap(&to), &1_i128, &None, &token));

    e.ledger().with_mut(|li| li.sequence_number = 100);
    assert!(client.can_transfer(&snap(&from), &snap(&to), &50_i128, &None, &token));

    client.on_transfer(&snap(&from), &snap(&to), &10_i128, &None, &token);
    assert_eq!(
        client.get_transfer_counter(&token, &from, &100_u32),
        TransferCounter { value: 10, deadline: 200 }
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #401)")]
fn on_transfer_panics_when_limit_exceeded() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let irs_id = e.register(MockIRSContract, ());

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_identity_registry_storage(&token, &irs_id, &manager);
    client.set_time_transfer_limit(
        &token,
        &TransferLimit { limit_duration: 100, limit_value: 50 },
        &manager,
    );

    client.on_transfer(&snap(&from), &snap(&to), &51_i128, &None, &token);
}

#[test]
fn can_transfer_true_when_no_limits_configured() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // No limits and no IRS: the identity lookup is skipped entirely.
    assert!(client.can_transfer(&snap(&from), &snap(&to), &100_i128, &None, &token));
}

#[test]
fn can_create_always_allows() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert!(client.can_create(&snap(&to), &100_i128, &token));
}

#[test]
fn name_returns_module_identifier() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.name(), String::from_str(&e, "TimeTransfersLimitsModule"));
}

#[test]
fn set_time_transfer_limit_requires_manager_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_time_transfer_limit(
        &token,
        &TransferLimit { limit_duration: 100, limit_value: 50 },
        &manager,
    );

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &manager);
}

#[test]
fn set_and_get_compliance_address_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let compliance = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);

    assert_eq!(client.get_compliance_address(&token), compliance);
}
