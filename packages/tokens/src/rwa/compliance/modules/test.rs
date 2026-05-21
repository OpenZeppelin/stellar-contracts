extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, IntoVal, Val,
    Vec,
};

use super::storage::*;
use crate::rwa::{
    identity_registry_storage::{
        CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
        IndividualCountryRelation,
    },
    utils::token_binder::TokenBinder,
};

#[contract]
struct MockModuleContract;

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

fn sample_country_entry() -> CountryData {
    CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(276)),
        metadata: None,
    }
}

#[test]
fn get_irs_client_returns_working_client_for_configured_token() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let account = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);

        let client = get_irs_client(&e, &token);
        assert_eq!(client.stored_identity(&account), account);
        assert_eq!(get_irs_country_data_entries(&e, &token, &account).len(), 0);
    });
}

#[test]
fn get_irs_country_data_entries_returns_typed_entries() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let account = Address::generate(&e);
    let entries = vec![&e, sample_country_entry()];

    e.as_contract(&irs_id, || {
        e.storage().persistent().set(&MockIRSStorageKey::CountryEntries(account.clone()), &entries);
    });

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);

        assert_eq!(get_irs_country_data_entries(&e, &token, &account), entries);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #396)")]
fn get_irs_client_panics_when_not_configured() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        let _ = get_irs_client(&e, &token);
    });
}

#[test]
fn panicking_math_helpers_return_expected_values() {
    let e = Env::default();

    assert_eq!(add_i128_or_panic(&e, 2, 3), 5);
    assert_eq!(sub_i128_or_panic(&e, 7, 4), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #391)")]
fn add_i128_or_panic_panics_on_overflow() {
    let e = Env::default();

    let _ = add_i128_or_panic(&e, i128::MAX, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #392)")]
fn sub_i128_or_panic_panics_on_underflow() {
    let e = Env::default();

    let _ = sub_i128_or_panic(&e, i128::MIN, 1);
}

#[test]
fn set_and_get_compliance_address_round_trip() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let token = Address::generate(&e);
    let compliance = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &token, &compliance);

        assert_eq!(get_compliance_address(&e, &token), compliance);
    });
}

#[test]
fn set_compliance_address_overwrites_existing_binding() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let token = Address::generate(&e);
    let first = Address::generate(&e);
    let second = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &token, &first);
        set_compliance_address(&e, &token, &second);

        assert_eq!(get_compliance_address(&e, &token), second);
    });
}

#[test]
fn compliance_address_is_isolated_per_token() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);
    let compliance_a = Address::generate(&e);
    let compliance_b = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &token_a, &compliance_a);
        set_compliance_address(&e, &token_b, &compliance_b);

        assert_eq!(get_compliance_address(&e, &token_a), compliance_a);
        assert_eq!(get_compliance_address(&e, &token_b), compliance_b);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #398)")]
fn get_compliance_address_panics_when_not_configured() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        let _ = get_compliance_address(&e, &token);
    });
}

#[test]
fn require_non_negative_amount_accepts_zero_and_positive() {
    let e = Env::default();

    require_non_negative_amount(&e, 0);
    require_non_negative_amount(&e, i128::MAX);
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn require_non_negative_amount_panics_on_negative() {
    let e = Env::default();

    require_non_negative_amount(&e, -1);
}
