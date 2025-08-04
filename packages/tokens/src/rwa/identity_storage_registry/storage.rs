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

use super::{
    emit_country_profile_event, emit_identity_modified, emit_identity_stored,
    emit_identity_unstored, CountryProfileEvent, IRSError, IDENTITY_EXTEND_AMOUNT,
    IDENTITY_TTL_THRESHOLD,
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

/// Key structure for enumerable country profile storage.
#[contracttype]
pub struct CPEnumerableKey {
    /// The account address that owns the country profile
    pub account: Address,
    /// The index position of the country profile in the enumerable list
    pub index: u32,
}

/// Storage keys for the data associated with Identity Storage Registry.
#[contracttype]
pub enum IRSStorageKey {
    /// Maps account address to identity address
    Identity(Address),
    /// Maps (account, index) to a specific CountryProfile
    CPEnumerable(CPEnumerableKey),
    /// Maps account address to the count of country profiles
    CPCount(Address),
}

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
/// * [`IRSError::EmptyCountryProfiles`] - If `initial_profiles` is empty.
/// * [`IRSError::IdentityAlreadyExists`] - If an identity is already stored for
///   the `account`.
/// * refer to [`add_country_profiles`] errors.
///
/// # Events
///
/// * topics - `["identity_stored", account: Address, identity: Address]`
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

    let key = IRSStorageKey::Identity(account.clone());
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::IdentityAlreadyExists)
    }
    e.storage().persistent().set(&key, identity);

    add_country_profiles(e, account, initial_profiles);

    emit_identity_stored(e, account, identity);
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

    emit_identity_unstored(e, account, &identity);
}

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

/// Retrieves the number of country profiles for a given account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
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

/// Retrieves all country profiles for a given account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
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
            // Unwrap should be always safe, if counting is done correctly,
            // adding "else" for consistency
            .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryProfileNotFound));
        profiles.push_back(profile);
    }

    profiles
}

/// Adds multiple country profiles to an existing identity.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to add profiles to.
/// * `profiles` - A vector of country profiles to add.
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
    let count_key = IRSStorageKey::CPCount(account.clone());
    let mut count = get_country_profile_count(e, account);

    for profile in profiles.iter() {
        let profile_key = IRSStorageKey::CPEnumerable(CPEnumerableKey {
            account: account.clone(),
            index: count, // Use the current count as the index for the new profile
        });
        e.storage().persistent().set(&profile_key, &profile);
        emit_country_profile_event(e, CountryProfileEvent::Added, account, &profile);
        count += 1;
    }
    e.storage().persistent().set(&count_key, &count);
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
    let key = IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index });

    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, IRSError::CountryProfileNotFound)
    }
    e.storage().persistent().set(&key, profile);

    emit_country_profile_event(e, CountryProfileEvent::Modified, account, profile);
}

/// Deletes a country profile by its index. This operation that swaps the
/// profile to be deleted with the last profile in the list.
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
/// * [`IRSError::EmptyCountryProfiles`] - If deleting the last profile is
///   attempted.
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
    let current_key =
        IRSStorageKey::CPEnumerable(CPEnumerableKey { account: account.clone(), index });
    let profile_to_remove = e
        .storage()
        .persistent()
        .get(&current_key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryProfileNotFound));

    let count = get_country_profile_count(e, account);
    // Can't overflow because `profile_to_remove` would panic if count == 0
    let last_index = count - 1;
    // Revert if no CountryProfile is left
    if last_index == 0 {
        panic_with_error!(e, IRSError::EmptyCountryProfiles)
    }

    // If the profile to be deleted is not the last one,
    // move the last profile into its place to keep the list compact.
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

    emit_country_profile_event(e, CountryProfileEvent::Removed, account, &profile_to_remove);
}
