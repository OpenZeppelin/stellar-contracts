extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, IntoVal,
    String, Val, Vec,
};
use stellar_tokens::rwa::{
    identity_registry_storage::{
        CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
        IndividualCountryRelation, OrganizationCountryRelation,
    },
    utils::token_binder::TokenBinder,
};

use crate::contract::{CountryRestrictContract, CountryRestrictContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> CountryRestrictContractClient<'a> {
    let address = e.register(CountryRestrictContract, (admin,));
    CountryRestrictContractClient::new(e, &address)
}

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
fn add_and_remove_country_restriction_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    assert!(!client.is_country_restricted(&token, &276));

    client.add_country_restriction(&token, &276);
    assert!(client.is_country_restricted(&token, &276));

    client.remove_country_restriction(&token, &276);
    assert!(!client.is_country_restricted(&token, &276));
}

#[test]
fn batch_restrict_and_unrestrict_countries_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.batch_restrict_countries(&token, &vec![&e, 250u32, 276u32]);
    assert!(client.is_country_restricted(&token, &250));
    assert!(client.is_country_restricted(&token, &276));

    client.batch_unrestrict_countries(&token, &vec![&e, 250u32]);
    assert!(!client.is_country_restricted(&token, &250));
    assert!(client.is_country_restricted(&token, &276));
}

#[test]
fn name_and_compliance_address_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let client = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "CountryRestrictModule"));

    client.set_compliance_address(&compliance);
    assert_eq!(client.get_compliance_address(), compliance);
}

#[test]
fn set_identity_registry_storage_uses_admin_auth_before_compliance_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let irs = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_identity_registry_storage(&token, &irs);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_identity_registry_storage_uses_compliance_auth_after_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let irs = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);

    client.set_identity_registry_storage(&token, &irs);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &compliance);
}

#[test]
fn can_transfer_and_can_create_use_irs_country_entries() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let from = Address::generate(&e);
    let token = Address::generate(&e);
    let allowed_to = Address::generate(&e);
    let restricted_to = Address::generate(&e);
    let amount = 100_i128;
    let client = create_client(&e, &admin);
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);

    irs.set_country_data_entries(&allowed_to, &vec![&e, individual_country(250)]);
    irs.set_country_data_entries(
        &restricted_to,
        &vec![&e, individual_country(250), organization_country(276)],
    );

    client.set_identity_registry_storage(&token, &irs_id);
    client.add_country_restriction(&token, &276);

    assert!(client.can_transfer(&from, &allowed_to, &amount, &token));
    assert!(client.can_create(&allowed_to, &amount, &token));
    assert!(!client.can_transfer(&from, &restricted_to, &amount, &token));
    assert!(!client.can_create(&restricted_to, &amount, &token));
}
