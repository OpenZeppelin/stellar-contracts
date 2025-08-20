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
/// - Category: A custom `Symbol` like `"TaxResidency"`, Code: `756` (ISO 3166-1
///   for Switzerland)
///
/// This flexible structure allows an account to hold multiple country profiles,
/// such as having dual citizenship. Additionally, each profile can have an
/// optional expiration date, which is useful for time-limited documents like
/// visas or permits.
///
/// When a new identity is registered for an account, it must be created with at
/// least one initial country profile. Afterward, more profiles can be added (up
/// to MAX_COUNTRY_PROFILES), modified, or removed as needed.
///
/// # Assumptions
///
/// 1. **All Profiles are Equal**: The system treats the initial profile and any
///    subsequently added profiles the same way. They are all stored together in
///    an enumerable list.
/// 2. **Efficient but Simple Indexing**: Profiles are stored by a simple index
///    (0, 1, 2, ...). When a profile is deleted, all subsequent profiles are
///    shifted to the left to fill the gap.
/// 3. **No Uniqueness Guarantee**: The storage layer itself does not check for
///    duplicate profiles. It is the responsibility of the contract implementing
///    the logic to ensure that, for example, an account does not have two
///    "Country of Residence" profiles.
///
/// # Example implementation of `CountryProfileManager` with uniqueness check
///
/// ```rust
/// #[contractimpl]
/// impl CountryProfileManager for MyContract {
///     fn add_country_profiles(
///         e: &Env,
///         account: Address,
///         country_profiles: Vec<Self::CountryProfile>,
///         operator: Address,
///     ) {
///         let existing_profiles = get_country_profiles(e, &account);
///
///         // Check each new profile for duplicates
///         for new_profile in country_profiles.iter() {
///             for existing in existing_profiles.iter() {
///                 // Maybe also check `valid_until`
///                 if existing.country == new_profile.country {
///                     panic_with_error!(e, Error::DuplicateCountryCategory);
///                 }
///             }
///         }
///
///         // If no duplicates found, add all profiles
///         add_country_profiles(e, &account, &country_profiles);
///     }
///     // other methods
/// }
/// ```
use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol, TryFromVal, Val, Vec};

use crate::rwa::identity_storage_registry::{
    emit_country_profile_event, emit_identity_modified, emit_identity_stored,
    emit_identity_unstored, CountryProfileEvent, IRSError, IDENTITY_EXTEND_AMOUNT,
    IDENTITY_TTL_THRESHOLD, MAX_COUNTRY_PROFILES,
};

/// ISO 3166-1 numeric country code
pub type CountryCode = u32;

/// Represents different types of country relationships.
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

/// A country profile containing the country relationship and optional validity
/// period.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryProfile {
    /// Type of country relationship
    pub country: Country,
    /// Optional validity period (e.g., for visas)
    pub valid_until: Option<u64>,
}

/// Storage keys for the data associated with Identity Storage Registry.
#[contracttype]
pub enum IRSStorageKey {
    /// Maps account address to identity address
    Identity(Address),
    /// Maps an account to a vector of its country profiles.
    CountryProfiles(Address),
}

// ################## QUERY STATE ##################

/// Retrieves the stored identity for a given account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
///
/// # Errors
///
/// * [`IRSError::IdentityNotFound`] - If no identity is found for the
///   `account`.
pub fn get_identity(e: &Env, account: &Address) -> Address {
    let key = IRSStorageKey::Identity(account.clone());
    get_persistent_entry(e, &key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound))
}

/// Retrieves a specific country profile by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
/// * `index` - The index of the country profile to retrieve.
///
/// # Errors
///
/// * [`IRSError::CountryProfileNotFound`] - If the index is out of bounds.
pub fn get_country_profile(e: &Env, account: &Address, index: u32) -> CountryProfile {
    let profiles = get_country_profiles(e, account);
    profiles.get(index).unwrap_or_else(|| panic_with_error!(e, IRSError::CountryProfileNotFound))
}

/// Retrieves all country profiles for a given account. Returns an empty vector
/// if not set.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
///
/// # Errors
///
/// * [`IRSError::EmptyCountryProfiles`] - If no country profiles are stored.
pub fn get_country_profiles(e: &Env, account: &Address) -> Vec<CountryProfile> {
    let key = IRSStorageKey::CountryProfiles(account.clone());
    get_persistent_entry(e, &key).unwrap_or_else(|| Vec::new(e))
}

// ################## CHANGE STATE ##################

/// Stores a new identity with a set of country profiles.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to associate with the identity.
/// * `identity` - The identity address to store.
/// * `initial_profiles` - A vector of initial country profiles.
///
/// # Errors
///
/// * [`IRSError::IdentityAlreadyExists`] - If an identity is already stored for
///   the `account`.
/// * [`IRSError::EmptyCountryProfiles`] - If `initial_profiles` is empty.
/// * [`IRSError::MaxCountryProfilesReached`] - If the number of
///   `initial_profiles` exceeds `MAX_COUNTRY_PROFILES`.
///
/// # Events
///
/// * topics - `["identity_stored", account: Address, identity: Address]`
/// * data - `[]`
///
/// Emits for each country profile added:
/// * topics - `["country_added", account: Address, profile: CountryProfile]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn add_identity(
    e: &Env,
    account: &Address,
    identity: &Address,
    initial_profiles: &Vec<CountryProfile>,
) {
    if initial_profiles.is_empty() {
        panic_with_error!(e, IRSError::EmptyCountryProfiles)
    }
    if initial_profiles.len() > MAX_COUNTRY_PROFILES {
        panic_with_error!(e, IRSError::MaxCountryProfilesReached);
    }

    let key = IRSStorageKey::Identity(account.clone());
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::IdentityAlreadyExists)
    }
    e.storage().persistent().set(&key, identity);

    emit_identity_stored(e, account, identity);

    e.storage()
        .persistent()
        .set(&IRSStorageKey::CountryProfiles(account.clone()), initial_profiles);

    for profile in initial_profiles.iter() {
        emit_country_profile_event(e, CountryProfileEvent::Added, account, &profile);
    }
}

/// Modifies an existing identity.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address whose identity is being modified.
/// * `new_identity` - The new identity address.
///
/// # Errors
///
/// * [`IRSError::IdentityNotFound`] - If no identity is found for the
///   `account`.
///
/// # Events
///
/// * topics - `["identity_modified", old_identity: Address, new_identity:
///   Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn modify_identity(e: &Env, account: &Address, new_identity: &Address) {
    let key = IRSStorageKey::Identity(account.clone());

    let old_identity: Address = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound));

    e.storage().persistent().set(&key, new_identity);

    emit_identity_modified(e, &old_identity, new_identity);
}

/// Removes an identity and all associated country profiles.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address whose identity is being removed.
///
/// # Errors
///
/// * [`IRSError::IdentityNotFound`] - If no identity is found for the
///   `account`.
///
/// # Events
///
/// * topics - `["identity_unstored", account: Address, identity: Address]`
/// * data - `[]`
///
/// Emits for each country profile removed:
/// * topics - `["country_removed", account: Address, profile: CountryProfile]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn remove_identity(e: &Env, account: &Address) {
    let key = IRSStorageKey::Identity(account.clone());

    let identity: Address = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound));
    e.storage().persistent().remove(&key);

    emit_identity_unstored(e, account, &identity);

    // Remove all associated country profiles
    let profiles_key = IRSStorageKey::CountryProfiles(account.clone());
    let profiles: Vec<CountryProfile> =
        e.storage().persistent().get(&profiles_key).expect("country profiles must be already set");
    e.storage().persistent().remove(&profiles_key);

    for profile in profiles {
        emit_country_profile_event(e, CountryProfileEvent::Removed, account, &profile);
    }
}

/// Adds multiple country profiles to an existing identity.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to add profiles to.
/// * `profiles` - A vector of country profiles to add.
///
/// # Errors
///
/// * [`IRSError::EmptyCountryProfiles`] - If `profiles` is empty.
/// * [`IRSError::MaxCountryProfilesReached`] - If the number of country
///   profiles exceeds `MAX_COUNTRY_PROFILES`.
///
/// # Events
///
/// Emits for each profile added:
/// * topics - `["country_added", account: Address, profile: CountryProfile]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn add_country_profiles(e: &Env, account: &Address, profiles: &Vec<CountryProfile>) {
    if profiles.is_empty() {
        panic_with_error!(e, IRSError::EmptyCountryProfiles)
    }

    let mut existing_profiles: Vec<CountryProfile> =
        get_persistent_entry(e, &IRSStorageKey::CountryProfiles(account.clone()))
            .expect("country profiles must be already set");

    existing_profiles.append(profiles);
    if existing_profiles.len() > MAX_COUNTRY_PROFILES {
        panic_with_error!(e, IRSError::MaxCountryProfilesReached);
    }

    let key = IRSStorageKey::CountryProfiles(account.clone());
    e.storage().persistent().set(&key, &existing_profiles);

    for profile in profiles.iter() {
        emit_country_profile_event(e, CountryProfileEvent::Added, account, &profile);
    }
}

/// Modifies an existing country profile by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address whose profile is being modified.
/// * `index` - The index of the profile to modify.
/// * `profile` - The new country profile data.
///
/// # Errors
///
/// * [`IRSError::CountryProfileNotFound`] - If the index is out of bounds.
///
/// # Events
///
/// * topics - `["country_modified", account: Address, profile: CountryProfile]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn modify_country_profile(e: &Env, account: &Address, index: u32, profile: &CountryProfile) {
    let mut profiles = get_country_profiles(e, account);
    if index >= profiles.len() {
        panic_with_error!(e, IRSError::CountryProfileNotFound);
    }
    profiles.set(index, profile.clone());

    let key = IRSStorageKey::CountryProfiles(account.clone());
    e.storage().persistent().set(&key, &profiles);

    emit_country_profile_event(e, CountryProfileEvent::Modified, account, profile);
}

/// Deletes a country profile by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address whose profile is being deleted.
/// * `index` - The index of the profile to delete.
///
/// # Errors
///
/// * [`IRSError::CountryProfileNotFound`] - If the index is out of bounds.
/// * [`IRSError::EmptyCountryProfiles`] - If attempting to delete the last
///   country profile.
///
/// # Events
///
/// * topics - `["country_removed", account: Address, profile: CountryProfile]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn delete_country_profile(e: &Env, account: &Address, index: u32) {
    let mut profiles = get_country_profiles(e, account);

    if profiles.len() == 1 {
        panic_with_error!(e, IRSError::EmptyCountryProfiles)
    }

    let profile_to_remove = profiles
        .get(index)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryProfileNotFound));

    profiles.remove(index);

    let key = IRSStorageKey::CountryProfiles(account.clone());
    e.storage().persistent().set(&key, &profiles);

    emit_country_profile_event(e, CountryProfileEvent::Removed, account, &profile_to_remove);
}

/// Helper function that tries to retrieve a persistent storage value and
/// extend its TTL if the entry exists.
///
/// # Arguments
///
/// * `e` - The Soroban reference.
/// * `key` - The key required to retrieve the underlying storage.
fn get_persistent_entry<T: TryFromVal<Env, Val>>(e: &Env, key: &IRSStorageKey) -> Option<T> {
    e.storage().persistent().get::<_, T>(key).inspect(|_| {
        e.storage().persistent().extend_ttl(key, IDENTITY_TTL_THRESHOLD, IDENTITY_EXTEND_AMOUNT);
    })
}
