#![cfg(test)]
extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use super::storage::{self as irs, CountryInfo, CountryProfile};

#[contract]
struct MockContract;

#[test]
fn add_identity_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);

        let initial_profile = CountryProfile {
            country: CountryInfo::Residence(840), // USA
            valid_until: None,
        };

        irs::add_identity(&e, &account, &identity, &initial_profile);

        assert_eq!(irs::get_identity(&e, &account), identity);
        assert_eq!(irs::get_country_profile_count(&e, &account), 1);
        assert_eq!(irs::get_country_profile(&e, &account, 0), initial_profile);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // IdentityAlreadyExists
fn add_identity_already_exists() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let identity = Address::generate(&e);

        let initial_profile =
            CountryProfile { country: CountryInfo::Residence(840), valid_until: None };

        irs::add_identity(&e, &account, &identity, &initial_profile);
        // Try to add it again
        irs::add_identity(&e, &account, &identity, &initial_profile);
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

        let initial_profile =
            CountryProfile { country: CountryInfo::Residence(840), valid_until: None };

        irs::add_identity(&e, &account, &old_identity, &initial_profile);
        irs::modify_identity(&e, &account, &new_identity);

        assert_eq!(irs::get_identity(&e, &account), new_identity);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // IdentityNotFound
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

        let initial_profile =
            CountryProfile { country: CountryInfo::Residence(840), valid_until: None };

        irs::add_identity(&e, &account, &identity, &initial_profile);

        assert_eq!(irs::get_identity(&e, &account), identity);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // IdentityNotFound
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
            country: CountryInfo::Residence(840), // USA
            valid_until: None,
        };
        let profile2 = CountryProfile {
            country: CountryInfo::Citizenship(276), // Germany
            valid_until: None,
        };

        irs::add_identity(&e, &account, &identity, &profile1);
        irs::add_country_profile(&e, &account, &profile2);

        assert_eq!(irs::get_country_profile_count(&e, &account), 2);

        irs::remove_identity(&e, &account);

        assert_eq!(irs::get_country_profile_count(&e, &account), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // IdentityNotFound
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
        let profile1 = CountryProfile {
            country: CountryInfo::Residence(840),
            valid_until: None,
        };
        let profile2 = CountryProfile {
            country: CountryInfo::Citizenship(276),
            valid_until: None,
        };

        irs::add_identity(&e, &account, &identity, &profile1);
        irs::add_country_profile(&e, &account, &profile2);

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
        let initial_profile = CountryProfile {
            country: CountryInfo::Residence(840),
            valid_until: None,
        };
        let modified_profile = CountryProfile {
            country: CountryInfo::Residence(276), // Germany
            valid_until: Some(12345),
        };

        irs::add_identity(&e, &account, &identity, &initial_profile);
        irs::modify_country_profile(&e, &account, 0, &modified_profile);

        assert_eq!(irs::get_country_profile(&e, &account, 0), modified_profile);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // CountryProfileNotFound
fn modify_country_profile_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let account = Address::generate(&e);
        let modified_profile = CountryProfile {
            country: CountryInfo::Residence(276),
            valid_until: None,
        };
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
        let profile1 = CountryProfile {
            country: CountryInfo::Residence(840),
            valid_until: None,
        };
        let profile2 = CountryProfile {
            country: CountryInfo::Citizenship(276),
            valid_until: None,
        };

        irs::add_identity(&e, &account, &identity, &profile1);
        irs::add_country_profile(&e, &account, &profile2);

        irs::delete_country_profile(&e, &account, 0);

        assert_eq!(irs::get_country_profile_count(&e, &account), 1);
        assert_eq!(irs::get_country_profile(&e, &account, 0), profile2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // CountryProfileNotFound
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
        let profile1 = CountryProfile {
            country: CountryInfo::Residence(840),
            valid_until: None,
        };
        let profile2 = CountryProfile {
            country: CountryInfo::Citizenship(276),
            valid_until: None,
        };

        irs::add_identity(&e, &account, &identity, &profile1);
        irs::add_country_profile(&e, &account, &profile2);

        let profiles = irs::get_country_profiles(&e, &account);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles.get(0).unwrap(), profile1);
        assert_eq!(profiles.get(1).unwrap(), profile2);
    });
}

#[test]
fn recover_country_profiles_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let old_account = Address::generate(&e);
        let new_account = Address::generate(&e);
        let identity = Address::generate(&e);
        let profile1 = CountryProfile {
            country: CountryInfo::Residence(840),
            valid_until: None,
        };
        let profile2 = CountryProfile {
            country: CountryInfo::Citizenship(276),
            valid_until: None,
        };

        irs::add_identity(&e, &old_account, &identity, &profile1);
        irs::add_country_profile(&e, &old_account, &profile2);

        irs::recover_country_profiles(&e, &old_account, &new_account);

        assert_eq!(irs::get_country_profile_count(&e, &old_account), 0);
        let new_profiles = irs::get_country_profiles(&e, &new_account);
        assert_eq!(new_profiles.len(), 2);
        assert_eq!(new_profiles.get(0).unwrap(), profile1);
        assert_eq!(new_profiles.get(1).unwrap(), profile2);
    });
}
