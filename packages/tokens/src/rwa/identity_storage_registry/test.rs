#![cfg(test)]
extern crate std;

use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env, Vec};

use super::{
    storage::{
        add_country_profiles, add_identity, delete_country_profile, get_country_profile,
        get_country_profiles, get_identity, modify_country_profile, modify_identity,
        remove_identity, Country, CountryProfile,
    },
    MAX_COUNTRY_PROFILES,
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
        let profile = CountryProfile {
            country: Country::Residence(840), // USA
            valid_until: None,
        };

        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);

        let stored_identity = get_identity(&e, &account);
        assert_eq!(stored_identity, identity);
        assert_eq!(get_country_profiles(&e, &account).len(), 1);
        assert_eq!(get_country_profile(&e, &account, 0), profile);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #320)")] // IdentityAlreadyExists
fn add_identity_already_exists() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile = CountryProfile {
            country: Country::Residence(840), // USA
            valid_until: None,
        };

        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
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
        let profile = CountryProfile {
            country: Country::Residence(840), // USA
            valid_until: None,
        };

        add_identity(&e, &account, &old_identity, &vec![&e, profile.clone()]);
        modify_identity(&e, &account, &new_identity);

        assert_eq!(get_identity(&e, &account), new_identity);
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
        let profile = CountryProfile {
            country: Country::Residence(840), // USA
            valid_until: None,
        };

        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);

        assert_eq!(get_identity(&e, &account), identity);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn get_identity_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        get_identity(&e, &account);
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
        let profile = CountryProfile {
            country: Country::Residence(840), // USA
            valid_until: None,
        };

        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);

        assert_eq!(get_country_profiles(&e, &account).len(), 1);

        remove_identity(&e, &account);

        get_identity(&e, &account);
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
fn add_country_profile_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile { country: Country::Residence(840), valid_until: None };
        let profile2 = CountryProfile { country: Country::Citizenship(276), valid_until: None };

        add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        add_country_profiles(&e, &account, &vec![&e, profile2.clone()]);

        assert_eq!(get_country_profiles(&e, &account).len(), 2);
        assert_eq!(get_country_profile(&e, &account, 1), profile2);
    });
}

#[test]
fn modify_country_profile_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let initial_profile =
            CountryProfile { country: Country::Residence(840), valid_until: None };
        let modified_profile = CountryProfile {
            country: Country::Residence(276), // Germany
            valid_until: Some(12345),
        };

        add_identity(&e, &account, &identity, &vec![&e, initial_profile.clone()]);
        modify_country_profile(&e, &account, 0, &modified_profile);

        assert_eq!(get_country_profile(&e, &account, 0), modified_profile);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #322)")] // CountryProfileNotFound
fn modify_country_profile_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile = CountryProfile { country: Country::Residence(276), valid_until: None };
        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
        modify_country_profile(&e, &account, 1, &profile);
    });
}

#[test]
fn delete_country_profile_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile { country: Country::Residence(840), valid_until: None };
        let profile2 = CountryProfile { country: Country::Citizenship(276), valid_until: None };
        let profile3 = CountryProfile { country: Country::Residence(4), valid_until: Some(123) };

        add_identity(
            &e,
            &account,
            &identity,
            &vec![&e, profile1.clone(), profile2.clone(), profile3.clone()],
        );

        // Delete the second profile (at index 1)
        delete_country_profile(&e, &account, 1);

        // Count should be 2, and profiles should have shifted left.
        assert_eq!(get_country_profiles(&e, &account).len(), 2);
        assert_eq!(get_country_profile(&e, &account, 0), profile1);
        assert_eq!(get_country_profile(&e, &account, 1), profile3);
    });
}

#[test]
fn get_country_profiles_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile { country: Country::Residence(840), valid_until: None };
        let profile2 = CountryProfile { country: Country::Citizenship(276), valid_until: None };

        add_identity(&e, &account, &identity, &vec![&e, profile1.clone(), profile2.clone()]);
        assert_eq!(get_country_profiles(&e, &account).len(), 2);

        // Deleting index 1 (the last profile)
        delete_country_profile(&e, &account, 1);

        assert_eq!(get_country_profiles(&e, &account).len(), 1);
        assert_eq!(get_country_profile(&e, &account, 0), profile1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #323)")] // EmptyCountryProfiles
fn get_country_profiles_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        get_country_profiles(&e, &account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #323)")] // EmptyCountryProfiles
fn add_identity_with_empty_profiles_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);

        add_identity(&e, &account, &identity, &vec![&e]);
    });
}

#[test]
fn add_multiple_country_profiles() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile { country: Country::Residence(840), valid_until: None };
        let profile2 = CountryProfile { country: Country::Citizenship(276), valid_until: None };
        let profile3 = CountryProfile { country: Country::Residence(4), valid_until: Some(123) };

        add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        add_country_profiles(&e, &account, &vec![&e, profile2.clone(), profile3.clone()]);

        assert_eq!(get_country_profiles(&e, &account).len(), 3);
        assert_eq!(get_country_profile(&e, &account, 0), profile1);
        assert_eq!(get_country_profile(&e, &account, 1), profile2);
        assert_eq!(get_country_profile(&e, &account, 2), profile3);
    });
}

#[test]
fn delete_last_country_profile() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile { country: Country::Residence(840), valid_until: None };
        let profile2 = CountryProfile { country: Country::Citizenship(276), valid_until: None };

        add_identity(&e, &account, &identity, &vec![&e, profile1.clone(), profile2.clone()]);
        assert_eq!(get_country_profiles(&e, &account).len(), 2);

        delete_country_profile(&e, &account, 1);

        assert_eq!(get_country_profiles(&e, &account).len(), 1);
        assert_eq!(get_country_profile(&e, &account, 0), profile1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #323)")] // EmptyCountryProfiles
fn delete_country_profile_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        delete_country_profile(&e, &account, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #323)")] // EmptyCountryProfiles
fn delete_last_country_profile_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile = CountryProfile { country: Country::Residence(840), valid_until: None };

        add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
        delete_country_profile(&e, &account, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #324)")] // MaxCountryProfilesReached
fn add_identity_panics_if_too_many_profiles() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let mut profiles = Vec::new(&e);
        for i in 0..=MAX_COUNTRY_PROFILES {
            profiles
                .push_back(CountryProfile { country: Country::Residence(i), valid_until: None });
        }

        add_identity(&e, &account, &identity, &profiles);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #324)")] // MaxCountryProfilesReached
fn add_country_profiles_panics_if_too_many_profiles() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let mut initial_profiles = Vec::new(&e);
        for i in 0..MAX_COUNTRY_PROFILES {
            initial_profiles
                .push_back(CountryProfile { country: Country::Residence(i), valid_until: None });
        }

        add_identity(&e, &account, &identity, &initial_profiles);

        let mut new_profiles = Vec::new(&e);
        new_profiles.push_back(CountryProfile {
            country: Country::Residence(MAX_COUNTRY_PROFILES),
            valid_until: None,
        });

        add_country_profiles(&e, &account, &new_profiles);
    });
}
