extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, IntoVal, Val,
    Vec,
};

use crate::rwa::{
    compliance::modules::{
        country_allow::storage::{can_receive, set_country_allowed},
        storage::set_irs_address,
    },
    identity_registry_storage::{
        CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
        IndividualCountryRelation, OrganizationCountryRelation,
    },
    utils::token_binder::TokenBinder,
};

#[contract]
struct MockIRSContract;

#[contracttype]
#[derive(Clone)]
enum MockIRSStorageKey {
    Identity(Address),
    CountryEntries(Address),
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

    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Val> {
        let entries: Vec<CountryData> = e
            .storage()
            .persistent()
            .get(&MockIRSStorageKey::CountryEntries(account))
            .unwrap_or_else(|| Vec::new(e));

        Vec::from_iter(e, entries.iter().map(|entry| entry.into_val(e)))
    }
}

#[contractimpl]
impl MockIRSContract {
    pub fn set_country_data_entries(e: &Env, account: Address, entries: Vec<CountryData>) {
        e.storage().persistent().set(&MockIRSStorageKey::CountryEntries(account), &entries);
    }
}

#[contract]
struct TestCountryAllowContract;

fn individual_country(code: u32) -> CountryData {
    CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(code)),
        metadata: None,
    }
}

fn organization_country(code: u32) -> CountryData {
    CountryData {
        country: CountryRelation::Organization(OrganizationCountryRelation::OperatingJurisdiction(
            code,
        )),
        metadata: None,
    }
}

#[test]
fn can_receive_allows_when_any_country_matches() {
    let e = Env::default();
    let module_id = e.register(TestCountryAllowContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    irs.set_country_data_entries(
        &to,
        &vec![&e, individual_country(250), organization_country(276)],
    );

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_country_allowed(&e, &token, 276);

        assert!(can_receive(&e, &to, &token));
    });
}

#[test]
fn can_receive_rejects_when_no_country_matches() {
    let e = Env::default();
    let module_id = e.register(TestCountryAllowContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let empty_to = Address::generate(&e);
    let disallowed_to = Address::generate(&e);

    irs.set_country_data_entries(&disallowed_to, &vec![&e, individual_country(250)]);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        set_country_allowed(&e, &token, 276);

        assert!(!can_receive(&e, &empty_to, &token));
        assert!(!can_receive(&e, &disallowed_to, &token));
    });
}
