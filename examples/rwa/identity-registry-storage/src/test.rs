extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
use stellar_tokens::rwa::identity_registry_storage::{
    CountryData, CountryRelation, IndividualCountryRelation,
};

use crate::contract::{IdentityRegistryContract, IdentityRegistryContractClient};

fn create_client(e: &Env) -> (Address, IdentityRegistryContractClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(IdentityRegistryContract, (&admin,));
    let client = IdentityRegistryContractClient::new(e, &contract_id);
    (admin, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);

    // Initially no tokens should be linked
    let tokens = client.linked_tokens();
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_token_binding() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);
    let token = Address::generate(&e);

    // Bind token
    client.bind_token(&token, &admin);
    let tokens = client.linked_tokens();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens.get(0).unwrap(), token);

    // Unbind token
    client.unbind_token(&token, &admin);
    assert_eq!(client.linked_tokens().len(), 0);
}

#[test]
fn test_identity_management() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);
    let account = Address::generate(&e);
    let identity = Address::generate(&e);

    let country_data = CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
        metadata: None,
    };
    let country_data_list = Vec::from_array(&e, [country_data.clone()]);

    // Add identity
    client.add_identity(&account, &identity, &country_data_list, &admin);

    // Check stored identity
    let stored_identity = client.stored_identity(&account);
    assert_eq!(stored_identity, identity);

    // Check country data
    let stored_country_data = client.get_country_data(&account, &0u32);
    assert_eq!(stored_country_data.country, country_data.country);
    assert_eq!(stored_country_data.metadata, country_data.metadata);

    // Get all country data entries
    let entries = client.get_country_data_entries(&account);
    assert_eq!(entries.len(), 1);

    // Remove identity
    client.remove_identity(&account, &admin);
}

#[test]
fn test_country_data_management() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);
    let account = Address::generate(&e);
    let identity = Address::generate(&e);

    let country_data = CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
        metadata: None,
    };
    let country_data_list = Vec::from_array(&e, [country_data.clone()]);

    // Add identity first
    client.add_identity(&account, &identity, &country_data_list, &admin);

    // Add more country data
    let new_country_data = CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(124)),
        metadata: None,
    };
    let new_country_data_list = Vec::from_array(&e, [new_country_data.clone()]);

    client.add_country_data_entries(&account, &new_country_data_list, &admin);

    // Should now have 2 entries
    let entries = client.get_country_data_entries(&account);
    assert_eq!(entries.len(), 2);

    // Modify country data
    let modified_country_data = CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(826)),
        metadata: None,
    };

    client.modify_country_data(&account, &1u32, &modified_country_data, &admin);
    let stored_data = client.get_country_data(&account, &1u32);
    assert_eq!(
        stored_data.country,
        CountryRelation::Individual(IndividualCountryRelation::Residence(826))
    );

    // Delete country data
    client.delete_country_data(&account, &1u32, &admin);
    let entries = client.get_country_data_entries(&account);
    assert_eq!(entries.len(), 1);
}
