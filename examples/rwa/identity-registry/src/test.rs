extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, FromVal, IntoVal};
use stellar_tokens::rwa::{
    identity_verification::identity_registry_storage::{
        CountryData, CountryRelation, IndividualCountryRelation,
    },
    utils::token_binder::MAX_TOKENS,
};

use crate::contract::{IdentityRegistryContract, IdentityRegistryContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> IdentityRegistryContractClient<'a> {
    let address = e.register(IdentityRegistryContract, (admin, manager));
    IdentityRegistryContractClient::new(e, &address)
}

#[test]
fn bind_max() {
    let e = Env::default();
    e.cost_estimate().disable_resource_limits();

    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();

    // The full capacity can be bound in a single call.
    let max_batch = MAX_TOKENS;
    let mut tokens = vec![&e];
    for _ in 0..max_batch {
        let token = Address::generate(&e);
        tokens.push_back(token.clone());
    }

    client.bind_tokens(&tokens, &manager);
    assert_eq!(client.linked_tokens().len(), max_batch)
}


#[test]
fn country_data_getters_exported() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();

    let account = Address::generate(&e);
    let identity = Address::generate(&e);
    let country_data = CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
        metadata: None,
    };
    let profiles = vec![&e, country_data.clone().into_val(&e)];

    client.add_identity(&account, &identity, &profiles, &manager);

    let entries = client.get_country_data_entries(&account);
    assert_eq!(entries.len(), 1);
    assert_eq!(CountryData::from_val(&e, &entries.get(0).unwrap()), country_data);
    assert_eq!(
        CountryData::from_val(&e, &client.get_country_data(&account, &0)),
        country_data
    );
}
