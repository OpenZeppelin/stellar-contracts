extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, String, Val,
    Vec,
};
use stellar_tokens::rwa::{
    compliance::{AccountSnapshot, TransferKind},
    identity_registry_storage::{CountryDataManager, IdentityRegistryStorage},
    utils::token_binder::TokenBinder,
};

use crate::contract::{MaxBalanceContract, MaxBalanceContractClient};

/// This module tracks a per-identity mirror and ignores the snapshot balance
/// and frozen amounts, so they are left at zero.
fn snap(address: &Address) -> AccountSnapshot {
    AccountSnapshot { address: address.clone(), balance: 0, frozen: 0 }
}

fn create_client<'a>(e: &Env, admin: &Address, manager: &Address) -> MaxBalanceContractClient<'a> {
    let address = e.register(MaxBalanceContract, (admin, manager));
    MaxBalanceContractClient::new(e, &address)
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
fn set_and_get_max_balance_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.get_max_balance(&token), 0);

    client.set_max_balance(&token, &500_i128, &manager);
    assert_eq!(client.get_max_balance(&token), 500);
}

#[test]
fn preset_and_batch_preset_id_balances_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let id_a = Address::generate(&e);
    let id_b = Address::generate(&e);
    let id_c = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.preset_id_balance(&token, &id_a, &100_i128, &manager);
    assert_eq!(client.get_id_balance(&token, &id_a), 100);

    client.batch_preset_id_balances(
        &token,
        &vec![&e, id_b.clone(), id_c.clone()],
        &vec![&e, 200_i128, 300_i128],
        &manager,
    );
    assert_eq!(client.get_id_balance(&token, &id_b), 200);
    assert_eq!(client.get_id_balance(&token, &id_c), 300);
}

#[test]
fn mark_preset_completed_flips_flag() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert!(!client.is_preset_completed(&token));
    client.mark_preset_completed(&token, &manager);
    assert!(client.is_preset_completed(&token));
}

#[test]
fn name_returns_module_identifier() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.name(), String::from_str(&e, "MaxBalanceModule"));
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

#[test]
fn set_compliance_address_requires_admin_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let compliance = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_max_balance_requires_manager_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_max_balance(&token, &100_i128, &manager);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #396)")]
fn on_transfer_panics_when_irs_not_configured() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_max_balance(&token, &100_i128, &manager);
    client.on_transfer(&snap(&from), &snap(&to), &10_i128, &TransferKind::Standard, &token);
}

#[test]
#[should_panic(expected = "Error(Contract, #393)")]
fn cap_applies_to_identity_aggregate() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet_a = Address::generate(&e);
    let wallet_b = Address::generate(&e);
    let shared_id = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);

    // Both wallets resolve to the same identity.
    irs.set_identity(&wallet_a, &shared_id);
    irs.set_identity(&wallet_b, &shared_id);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_identity_registry_storage(&token, &irs_id, &manager);
    client.set_max_balance(&token, &100_i128, &manager);
    client.preset_id_balance(&token, &shared_id, &50_i128, &manager);

    // shared_id at 50: minting the headroom on one wallet succeeds, one
    // token more on the other wallet breaches the shared cap.
    client.on_created(&snap(&wallet_a), &50_i128, &token);
    client.on_created(&snap(&wallet_b), &1_i128, &token);
}

#[test]
fn on_created_and_on_destroyed_track_aggregate_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let wallet = Address::generate(&e);
    let identity = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);

    irs.set_identity(&wallet, &identity);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_identity_registry_storage(&token, &irs_id, &manager);
    client.set_max_balance(&token, &100_i128, &manager);

    client.on_created(&snap(&wallet), &40_i128, &token);
    client.on_created(&snap(&wallet), &30_i128, &token);
    assert_eq!(client.get_id_balance(&token, &identity), 70);

    client.on_destroyed(&snap(&wallet), &20_i128, &token);
    assert_eq!(client.get_id_balance(&token, &identity), 50);
}

#[test]
#[should_panic(expected = "Error(Contract, #395)")]
fn preset_id_balance_panics_after_completed() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let identity = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.mark_preset_completed(&token, &manager);
    client.preset_id_balance(&token, &identity, &1_i128, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #397)")]
fn batch_preset_id_balances_rejects_length_mismatch() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let id_a = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.batch_preset_id_balances(&token, &vec![&e, id_a], &vec![&e, 1_i128, 2_i128], &manager);
}
