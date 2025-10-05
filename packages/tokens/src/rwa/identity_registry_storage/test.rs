extern crate std;

use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env, Map, String, Symbol, Vec};

use super::{
    storage::{
        add_country_data_entries, add_identity, delete_country_data, get_country_data,
        get_country_data_entries, get_identity_profile, modify_country_data, modify_identity,
        recover_identity, remove_identity, stored_identity, CountryData, CountryRelation,
        IdentityType, IndividualCountryRelation, OrganizationCountryRelation,
    },
    MAX_COUNTRY_ENTRIES,
};

#[contract]
struct MockContract;

#[test]
fn add_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)), // USA
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );

        let stored_identity = stored_identity(&e, &account);
        assert_eq!(stored_identity, identity);

        let profile = get_identity_profile(&e, &account);
        assert_eq!(profile.identity_type, IdentityType::Individual);
        assert_eq!(profile.countries.len(), 1);
        assert_eq!(get_country_data(&e, &account, 0), country_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #320)")] // IdentityOverwrite
fn add_identity_already_exists() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)), // USA
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );
        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );
    });
}

#[test]
fn modify_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let old_identity = Address::generate(&e);
        let new_identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)), // USA
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &old_identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );
        modify_identity(&e, &account, &new_identity);

        assert_eq!(stored_identity(&e, &account), new_identity);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn modify_identity_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let new_identity = Address::generate(&e);

        modify_identity(&e, &account, &new_identity);
    });
}

#[test]
fn get_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)), // USA
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );

        assert_eq!(stored_identity(&e, &account), identity);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn get_identity_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        stored_identity(&e, &account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn remove_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)), // USA
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );

        assert_eq!(get_country_data_entries(&e, &account).len(), 1);

        remove_identity(&e, &account);

        stored_identity(&e, &account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn remove_identity_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        remove_identity(&e, &account);
    });
}

#[test]
fn add_country_data_entries_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data1 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let country_data2 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data1.clone()],
        );
        add_country_data_entries(&e, &account, &vec![&e, country_data2.clone()]);

        assert_eq!(get_country_data_entries(&e, &account).len(), 2);
        assert_eq!(get_country_data(&e, &account, 1), country_data2);
    });
}

#[test]
fn modify_country_data_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let initial_country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let mut metadata = Map::new(&e);
        metadata.set(Symbol::new(&e, "valid_until"), String::from_str(&e, "12345"));
        let modified_country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(276)), // Germany
            metadata: Some(metadata),
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, initial_country_data.clone()],
        );
        modify_country_data(&e, &account, 0, &modified_country_data);

        assert_eq!(get_country_data(&e, &account, 0), modified_country_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #322)")] // CountryDataNotFound
fn modify_country_data_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(276)),
            metadata: None,
        };
        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );
        modify_country_data(&e, &account, 1, &country_data);
    });
}

#[test]
fn delete_country_data_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data1 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let country_data2 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };
        let mut metadata = Map::new(&e);
        metadata.set(Symbol::new(&e, "valid_until"), String::from_str(&e, "123"));
        let country_data3 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(4)),
            metadata: Some(metadata),
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data1.clone(), country_data2.clone(), country_data3.clone()],
        );

        // Delete the second country data (at index 1)
        delete_country_data(&e, &account, 1);

        // Count should be 2, and country data should have shifted left.
        assert_eq!(get_country_data_entries(&e, &account).len(), 2);
        assert_eq!(get_country_data(&e, &account, 0), country_data1);
        assert_eq!(get_country_data(&e, &account, 1), country_data3);
    });
}

#[test]
fn get_country_data_entries_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data1 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let country_data2 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data1.clone(), country_data2.clone()],
        );
        assert_eq!(get_country_data_entries(&e, &account).len(), 2);

        // Deleting index 1 (the last country data)
        delete_country_data(&e, &account, 1);

        assert_eq!(get_country_data_entries(&e, &account).len(), 1);
        assert_eq!(get_country_data(&e, &account, 0), country_data1);
    });
}

#[test]
fn get_empty_country_data_list() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        assert_eq!(get_country_data_entries(&e, &account).len(), 0);
    });
}

#[test]
fn add_multiple_country_data() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data1 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let country_data2 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };
        let mut metadata = Map::new(&e);
        metadata.set(Symbol::new(&e, "valid_until"), String::from_str(&e, "123"));
        let country_data3 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(4)),
            metadata: Some(metadata),
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data1.clone()],
        );
        add_country_data_entries(
            &e,
            &account,
            &vec![&e, country_data2.clone(), country_data3.clone()],
        );

        assert_eq!(get_country_data_entries(&e, &account).len(), 3);
        assert_eq!(get_country_data(&e, &account, 0), country_data1);
        assert_eq!(get_country_data(&e, &account, 1), country_data2);
        assert_eq!(get_country_data(&e, &account, 2), country_data3);
    });
}

#[test]
fn delete_last_country_data() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data1 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let country_data2 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data1.clone(), country_data2.clone()],
        );
        assert_eq!(get_country_data_entries(&e, &account).len(), 2);

        delete_country_data(&e, &account, 1);

        assert_eq!(get_country_data_entries(&e, &account).len(), 1);
        assert_eq!(get_country_data(&e, &account, 0), country_data1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")]
fn delete_country_data_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        delete_country_data(&e, &account, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #323)")]
fn delete_last_country_data_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };

        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );
        delete_country_data(&e, &account, 0);
    });
}

#[test]
fn organization_country_relations_work_correctly() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let incorporation_data = CountryData {
            country: CountryRelation::Organization(OrganizationCountryRelation::Incorporation(840)), // USA
            metadata: None,
        };
        let operating_data = CountryData {
            country: CountryRelation::Organization(
                OrganizationCountryRelation::OperatingJurisdiction(276),
            ), // Germany
            metadata: None,
        };
        let tax_data = CountryData {
            country: CountryRelation::Organization(OrganizationCountryRelation::TaxJurisdiction(
                756,
            )), // Switzerland
            metadata: None,
        };

        // Create organization identity with incorporation data
        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Organization,
            &vec![&e, incorporation_data.clone()],
        );

        // Add more organization country data
        add_country_data_entries(&e, &account, &vec![&e, operating_data.clone(), tax_data.clone()]);

        // Verify all data is stored correctly
        let profile = get_identity_profile(&e, &account);
        assert_eq!(profile.identity_type, IdentityType::Organization);
        assert_eq!(profile.countries.len(), 3);
        assert_eq!(get_country_data(&e, &account, 0), incorporation_data);
        assert_eq!(get_country_data(&e, &account, 1), operating_data);
        assert_eq!(get_country_data(&e, &account, 2), tax_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #324)")]
fn add_identity_panics_if_too_many_country_data() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let mut country_data_list = Vec::new(&e);
        for i in 0..=MAX_COUNTRY_ENTRIES {
            country_data_list.push_back(CountryData {
                country: CountryRelation::Individual(IndividualCountryRelation::Residence(i)),
                metadata: None,
            });
        }

        add_identity(&e, &account, &identity, IdentityType::Individual, &country_data_list);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #324)")]
fn add_country_data_entries_panics_if_too_many() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let mut initial_country_data = Vec::new(&e);
        for i in 0..MAX_COUNTRY_ENTRIES {
            initial_country_data.push_back(CountryData {
                country: CountryRelation::Individual(IndividualCountryRelation::Residence(i)),
                metadata: None,
            });
        }

        add_identity(&e, &account, &identity, IdentityType::Individual, &initial_country_data);

        let mut new_country_data = Vec::new(&e);
        new_country_data.push_back(CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(
                MAX_COUNTRY_ENTRIES,
            )),
            metadata: None,
        });

        add_country_data_entries(&e, &account, &new_country_data);
    });
}

#[test]
fn modify_country_data_matching_type_succeeds() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let initial_country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let modified_country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };

        // First create an individual identity
        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![&e, initial_country_data],
        );

        // This should succeed: modifying to another individual country relation
        modify_country_data(&e, &account, 0, &modified_country_data);

        assert_eq!(get_country_data(&e, &account, 0), modified_country_data);
    });
}

#[test]
fn mixed_country_relations_succeeds() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let incorporation_data = CountryData {
            country: CountryRelation::Organization(OrganizationCountryRelation::Incorporation(840)),
            metadata: None,
        };
        let individual_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(276)),
            metadata: None,
        };

        // This should succeed: mixed country relation types for KYB compliance
        add_identity(
            &e,
            &account,
            &identity,
            IdentityType::Organization,
            &vec![&e, incorporation_data.clone(), individual_data.clone()],
        );

        let profile = get_identity_profile(&e, &account);
        assert_eq!(profile.identity_type, IdentityType::Organization);
        assert_eq!(profile.countries.len(), 2);
        assert_eq!(get_country_data(&e, &account, 0), incorporation_data);
        assert_eq!(get_country_data(&e, &account, 1), individual_data);
    });
}

#[test]
fn recover_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let old_account = Address::generate(&e);
        let new_account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data1 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };
        let country_data2 = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Citizenship(276)),
            metadata: None,
        };

        // Add identity to old account
        add_identity(
            &e,
            &old_account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data1.clone(), country_data2.clone()],
        );

        // Recover identity to new account
        recover_identity(&e, &old_account, &new_account);

        // Verify identity is now linked to new account
        assert_eq!(stored_identity(&e, &new_account), identity);

        // Verify identity profile is transferred
        let profile = get_identity_profile(&e, &new_account);
        assert_eq!(profile.identity_type, IdentityType::Individual);
        assert_eq!(profile.countries.len(), 2);
        assert_eq!(get_country_data(&e, &new_account, 0), country_data1);
        assert_eq!(get_country_data(&e, &new_account, 1), country_data2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn recover_identity_old_account_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let old_account = Address::generate(&e);
        let new_account = Address::generate(&e);

        // Try to recover identity from account that doesn't have one
        recover_identity(&e, &old_account, &new_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #320)")] // IdentityOverwrite
fn recover_identity_new_account_already_has_identity() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let old_account = Address::generate(&e);
        let new_account = Address::generate(&e);
        let identity1 = Address::generate(&e);
        let identity2 = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };

        // Add identity to old account
        add_identity(
            &e,
            &old_account,
            &identity1,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );

        // Add identity to new account
        add_identity(
            &e,
            &new_account,
            &identity2,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );

        // Try to recover identity to account that already has one
        recover_identity(&e, &old_account, &new_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn recover_identity_removes_old_account_identity() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let old_account = Address::generate(&e);
        let new_account = Address::generate(&e);
        let identity = Address::generate(&e);
        let country_data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
            metadata: None,
        };

        // Add identity to old account
        add_identity(
            &e,
            &old_account,
            &identity,
            IdentityType::Individual,
            &vec![&e, country_data.clone()],
        );

        // Recover identity to new account
        recover_identity(&e, &old_account, &new_account);

        // Verify old account no longer has identity (should panic)
        stored_identity(&e, &old_account);
    });
}
