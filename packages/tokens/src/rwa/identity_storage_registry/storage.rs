/// # How Country Profiles Work
///
/// Instead of a simple, single country code, this system treats an account's
/// jurisdictional ties as a collection of "Country Profiles." Each profile
/// represents a single piece of jurisdictional data, pairing a **category**
/// with a **numeric code**. For example:
///
/// - Category: `Residence`, Code: `840` (ISO 3166-1 for USA)
/// - Category: `Citizenship`, Code: `276` (ISO 3166-1 for Germany)
/// - Category: `SourceOfFunds`, Code: `792` (ISO 3166-1 for Turkey)
/// - Category: A custom `Symbol` like `"TaxResidency"`, Code: `756`
///   (ISO 3166-1 for Switzerland)
///
/// This flexible structure allows an account to hold multiple country profiles,
/// such as having dual citizenship. Additionally, each profile can have an
/// optional expiration date, which is useful for time-limited documents like
/// visas or permits.
///
/// When a new identity is registered for an account, it must be created with at
/// least one initial country profile. Afterward, more profiles can be added,
/// modified, or removed as needed.
///
/// # Assumptions
///
/// 1. **All Profiles are Equal**: The system treats the initial profile and any
///    subsequently added profiles the same way. They are all stored together in
///    an enumerable list.
/// 2. **Efficient but Simple Indexing**: Profiles are stored by a simple index
///    (0, 1, 2, ...). When a profile is deleted, the last profile in the list
///    is moved into its place to keep the list compact and gas-efficient. This
///    means the index of a profile can change.
/// 3. **No Uniqueness Guarantee**: The storage layer itself does not check for
///    duplicate profiles. It is the responsibility of the contract implementing
///    the logic to ensure that, for example, an account does not have two
///    "Country of Residence" profiles.
use soroban_sdk::{contracttype, panic_with_error, vec, Address, Env, Symbol, Vec};

use super::{IRSError, IDENTITY_EXTEND_AMOUNT, IDENTITY_TTL_THRESHOLD};

/// ISO 3166-1 numeric country code
pub type CountryCode = u32;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Country {
    /// Country of residence
    Residence(CountryCode),
    /// Country of citizenship
    Citizenship(CountryCode),
    /// Country where funds originate
    SourceOfFunds(CountryCode),
    /// Country of entity registration (for businesses)
    EntityJurisdiction(CountryCode),
    /// Custom country type for future extensions
    Custom(Symbol, CountryCode),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryProfile {
    /// Type of country relationship
    pub country: Country,
    /// Optional validity period (e.g., for visas)
    pub valid_until: Option<u64>,
}

#[contracttype]
pub struct CPEnumerableKey {
    pub account: Address,
    pub index: u32,
}

#[contracttype]
pub enum IRSStorageKey {
    Identity(Address),             // account -> Identity
    CPEnumerable(CPEnumerableKey), // (account, index) -> CountryProfile
    CPCount(Address),              // account -> number of enumerable CountryProfile
}

pub fn add_identity(
    e: &Env,
    account: &Address,
    identity: &Address,
    initial_profiles: &Vec<CountryProfile>,
) {
    if initial_profiles.is_empty() {
        panic_with_error!(e, IRSError::EmptyCountryProfiles)
    }

    let key = IRSStorageKey::Identity(account.clone());
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::IdentityAlreadyExists)
    }
    e.storage().persistent().set(&key, identity);

    add_country_profiles(e, account, initial_profiles);
}

pub fn modify_identity(e: &Env, account: &Address, new_identity: &Address) {
    let key = IRSStorageKey::Identity(account.clone());

    // check if identity exists
    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::IdentityNotFound)
    }

    e.storage().persistent().set(&key, new_identity);
}

pub fn remove_identity(e: &Env, account: &Address) {
    let key = IRSStorageKey::Identity(account.clone());

    // check if identity exists
    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::IdentityNotFound)
    }
    e.storage().persistent().remove(&key);

    // Remove all associated country profiles
    let count = get_country_profile_count(e, account);
    for i in 0..count {
        let profile_key =
            IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index: i });
        e.storage().persistent().remove(&profile_key);
    }

    // Remove the count
    let count_key = IRSStorageKey::CPCount(account.clone());
    e.storage().persistent().remove(&count_key);
}

pub fn get_identity(e: &Env, account: &Address) -> Address {
    let key = IRSStorageKey::Identity(account.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                IDENTITY_TTL_THRESHOLD,
                IDENTITY_EXTEND_AMOUNT,
            )
        })
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound))
}

// ================ Country Profile ===================

pub fn get_country_profile(e: &Env, account: &Address, index: u32) -> CountryProfile {
    let key = IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index });
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                IDENTITY_TTL_THRESHOLD,
                IDENTITY_EXTEND_AMOUNT,
            )
        })
        .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryProfileNotFound))
}

pub fn get_country_profile_count(e: &Env, account: &Address) -> u32 {
    let key = IRSStorageKey::CPCount(account.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                IDENTITY_TTL_THRESHOLD,
                IDENTITY_EXTEND_AMOUNT,
            )
        })
        .unwrap_or_default()
}

pub fn get_country_profiles(e: &Env, account: &Address) -> Vec<CountryProfile> {
    let count = get_country_profile_count(e, account);
    let mut profiles: Vec<CountryProfile> = vec![e];

    for index in 0..count {
        let key = IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index });
        let profile: CountryProfile = e
            .storage()
            .persistent()
            .get(&key)
            .inspect(|_| {
                e.storage().persistent().extend_ttl(
                    &key,
                    IDENTITY_TTL_THRESHOLD,
                    IDENTITY_EXTEND_AMOUNT,
                )
            })
            .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryProfileNotFound));
        profiles.push_back(profile);
    }

    profiles
}

pub fn add_country_profiles(e: &Env, account: &Address, profiles: &Vec<CountryProfile>) {
    let count_key = IRSStorageKey::CPCount(account.clone());
    let mut count = get_country_profile_count(e, account);

    for profile in profiles.iter() {
        let profile_key = IRSStorageKey::CPEnumerable(CPEnumerableKey {
            account: account.clone(),
            index: count, // Use the current count as the index for the new profile
        });
        e.storage().persistent().set(&profile_key, &profile);
        count += 1;
    }
    e.storage().persistent().set(&count_key, &count);
}

pub fn modify_country_profile(e: &Env, account: &Address, index: u32, profile: &CountryProfile) {
    let key = IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index });

    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::CountryProfileNotFound)
    }
    e.storage().persistent().set(&key, profile);
}

pub fn delete_country_profile(e: &Env, account: &Address, index: u32) {
    let count = get_country_profile_count(e, account);
    if index >= count {
        panic_with_error!(e, IRSError::CountryProfileNotFound)
    }

    // If the profile to be deleted is not the last one,
    // move the last profile into its place to keep the list compact.
    let last_index = count
        .checked_sub(1)
        // revert if no CountryProfile is left
        .unwrap_or_else(|| panic_with_error!(e, IRSError::EmptyCountryProfiles));
    if index != last_index {
        let last_key = IRSStorageKey::CPEnumerable(CPEnumerableKey {
            account: account.clone(),
            index: last_index,
        });
        let last_profile: CountryProfile = e
            .storage()
            .persistent()
            .get(&last_key)
            .inspect(|_| {
                e.storage().persistent().extend_ttl(
                    &last_key,
                    IDENTITY_TTL_THRESHOLD,
                    IDENTITY_EXTEND_AMOUNT,
                )
            })
            .unwrap_or_else(|| panic_with_error!(&e, IRSError::CountryProfileNotFound));

        let current_key =
            IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index });
        e.storage().persistent().set(&current_key, &last_profile);
    }

    // Remove the last profile's storage entry
    let key_to_remove = IRSStorageKey::CPEnumerable(CPEnumerableKey {
        account: account.clone(),
        index: last_index,
    });
    e.storage().persistent().remove(&key_to_remove);

    // Decrement the count
    e.storage().persistent().set(&IRSStorageKey::CPCount(account.clone()), &(count - 1));
}

pub fn recover_country_profiles(e: &Env, old_account: &Address, new_account: &Address) {
    let profiles = get_country_profiles(e, old_account);
    let count = profiles.len();

    if count == 0 {
        return;
    }

    // Move profiles to the new account
    for (i, profile) in profiles.iter().enumerate() {
        let new_profile_key = IRSStorageKey::CPEnumerable(CPEnumerableKey {
            account: new_account.clone(),
            index: i as u32,
        });
        e.storage().persistent().set(&new_profile_key, &profile);
    }

    // Set the count for the new account
    let new_count_key = IRSStorageKey::CPCount(new_account.clone());
    e.storage().persistent().set(&new_count_key, &count);

    // Remove all profiles from the old account
    for i in 0..count {
        let old_profile_key =
            IRSStorageKey::CPEnumerable(CPEnumerableKey { account: old_account.clone(), index: i });
        e.storage().persistent().remove(&old_profile_key);
    }

    // Remove the count for the old account
    let old_count_key = IRSStorageKey::CPCount(old_account.clone());
    e.storage().persistent().remove(&old_count_key);
}
