#![cfg(test)]
extern crate std;

use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env};

use super::storage::{self as irs, Country, CountryProfile};

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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);

        let stored_identity = irs::get_identity(&e, &account);
        assert_eq!(stored_identity, identity);
        assert_eq!(irs::get_country_profile_count(&e, &account), 1);
        assert_eq!(irs::get_country_profile(&e, &account, 0), profile);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
        irs::add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
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

        irs::add_identity(&e, &account, &old_identity, &vec![&e, profile.clone()]);
        irs::modify_identity(&e, &account, &new_identity);

        assert_eq!(irs::get_identity(&e, &account), new_identity);
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

        irs::modify_identity(&e, &account, &new_identity);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);

        assert_eq!(irs::get_identity(&e, &account), identity);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn get_identity_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        irs::get_identity(&e, &account);
    });
}

#[test]
fn remove_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile {
            country: Country::Residence(840), // USA
            valid_until: None,
        };
        let profile2 = CountryProfile {
            country: Country::Citizenship(276), // Germany
            valid_until: None,
        };

        irs::add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        irs::add_country_profiles(&e, &account, &vec![&e, profile2.clone()]);

        assert_eq!(irs::get_country_profile_count(&e, &account), 2);

        irs::remove_identity(&e, &account);

        assert_eq!(irs::get_country_profile_count(&e, &account), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #321)")] // IdentityNotFound
fn remove_identity_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        irs::remove_identity(&e, &account);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        irs::add_country_profiles(&e, &account, &vec![&e, profile2.clone()]);

        assert_eq!(irs::get_country_profile_count(&e, &account), 2);
        assert_eq!(irs::get_country_profile(&e, &account, 1), profile2);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, initial_profile.clone()]);
        irs::modify_country_profile(&e, &account, 0, &modified_profile);

        assert_eq!(irs::get_country_profile(&e, &account, 0), modified_profile);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #322)")] // CountryProfileNotFound
fn modify_country_profile_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let modified_profile =
            CountryProfile { country: Country::Residence(276), valid_until: None };
        irs::modify_country_profile(&e, &account, 0, &modified_profile);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        irs::add_country_profiles(&e, &account, &vec![&e, profile2.clone()]);

        irs::delete_country_profile(&e, &account, 0);

        assert_eq!(irs::get_country_profile_count(&e, &account), 1);
        assert_eq!(irs::get_country_profile(&e, &account, 0), profile2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #322)")] // CountryProfileNotFound
fn delete_country_profile_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        irs::delete_country_profile(&e, &account, 0);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        irs::add_country_profiles(&e, &account, &vec![&e, profile2.clone()]);

        let profiles = irs::get_country_profiles(&e, &account);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles.get(0).unwrap(), profile1);
        assert_eq!(profiles.get(1).unwrap(), profile2);
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

        irs::add_identity(&e, &account, &identity, &vec![&e]);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile1.clone()]);
        irs::add_country_profiles(&e, &account, &vec![&e, profile2.clone(), profile3.clone()]);

        assert_eq!(irs::get_country_profile_count(&e, &account), 3);
        assert_eq!(irs::get_country_profile(&e, &account, 0), profile1);
        assert_eq!(irs::get_country_profile(&e, &account, 1), profile2);
        assert_eq!(irs::get_country_profile(&e, &account, 2), profile3);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile1.clone(), profile2.clone()]);
        assert_eq!(irs::get_country_profile_count(&e, &account), 2);

        irs::delete_country_profile(&e, &account, 0);

        assert_eq!(irs::get_country_profile_count(&e, &account), 1);
        assert_eq!(irs::get_country_profile(&e, &account, 0), profile2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #322)")]
fn delete_country_profile_0_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        irs::delete_country_profile(&e, &account, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #322)")]
fn delete_country_profile_1_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        irs::delete_country_profile(&e, &account, 1);
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

        irs::add_identity(&e, &account, &identity, &vec![&e, profile.clone()]);
        irs::delete_country_profile(&e, &account, 0);
    });
}
