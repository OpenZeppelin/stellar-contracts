/// # How Country Data Work
///
/// Instead of a simple, single country code, this system treats an account's
/// jurisdictional ties as a collection of "Country Data." Each country data
/// represents a single piece of jurisdictional data, pairing a **relationship
/// type** with a **numeric country code**. For example:
///
/// For Individual identities:
/// - `Residence(840)` - Country of residence: USA
/// - `Citizenship(276)` - Country of citizenship: Germany
/// - `SourceOfFunds(792)` - Source of funds: Turkey
///
/// For Organization identities:
/// - `Incorporation(840)` - Country of incorporation: USA
/// - `OperatingJurisdiction(276)` - Operating jurisdiction: Germany
/// - `TaxJurisdiction(756)` - Tax jurisdiction: Switzerland
/// - `Custom("Subsidiary".into(), 792)` - Custom subsidiary location: Turkey
///
/// This flexible structure allows an account to hold multiple country
/// relationships, such as an individual having dual citizenship or an
/// organization operating across multiple jurisdictions. The system enforces
/// type matching between the identity type and country relation types.
///
/// When a new identity is registered for an account, it must be created with at
/// least one initial country data. Afterward, more country data can be added
/// (up to MAX_COUNTRY_ENTRIES), modified, or removed as needed.
///
/// ## Assumptions
///
/// 1. **All Country Data are Equal**: The system treats the initial country
///    data and any subsequently added country data the same way. They are all
///    stored together in an enumerable list.
/// 2. **Efficient but Simple Indexing**: Country data are stored by a simple
///    index (0, 1, 2, ...). When a country data is deleted, all subsequent
///    country data are shifted to the left to fill the gap.
/// 3. **No Uniqueness Guarantee**: The storage layer itself does not check for
///    duplicate country data. It is the responsibility of the contract
///    implementing the logic to ensure that, for example, an account does not
///    have two "Country of Residence" country data.
/// 4. **Country Data Type Matching**: All country data entries must match the
///    identity's type (Individual or Organization). Individual identities can
///    only have IndividualCountryRelation entries, while Organization
///    identities can only have OrganizationCountryRelation entries.
///
/// ### Example implementation of `CountryDataManager` with uniqueness check
///
/// ```rust
/// #[contractimpl]
/// impl CountryDataManager for MyContract {
///     fn add_country_data_entries(
///         e: &Env,
///         account: Address,
///         country_entries: Vec<Self::CountryData>,
///         operator: Address,
///     ) {
///         let existing = get_country_data_entries(e, &account);
///
///         // Check each new entries for duplicates
///         for new_entry in country_entries.iter() {
///             for existing in existing.iter() {
///                 // Maybe also check validity from metadata
///                 if existing.country == new_entry.country {
///                     panic_with_error!(e, Error::DuplicateCountryData);
///                 }
///             }
///         }
///
///         // If no duplicates found, add all entries
///         add_country_data_entries(e, &account, &country_entries);
///     }
///     // other methods
/// }
/// ```
use soroban_sdk::{
    contracttype, panic_with_error, vec, Address, Env, Map, String, Symbol, TryFromVal, Val, Vec,
};

use crate::rwa::identity_registry_storage::{
    emit_country_data_event, emit_identity_modified, emit_identity_stored, emit_identity_unstored,
    CountryDataEvent, IRSError, IDENTITY_EXTEND_AMOUNT, IDENTITY_TTL_THRESHOLD,
    MAX_COUNTRY_ENTRIES,
};

/// ISO 3166-1 numeric country code
pub type CountryCode = u32;

/// Represents the type of identity holder
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityType {
    Individual,
    Organization,
}

/// Represents different types of country relationships for individuals
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndividualCountryRelation {
    /// Country of residence
    Residence(CountryCode),
    /// Country of citizenship
    Citizenship(CountryCode),
    /// Country where funds originate
    SourceOfFunds(CountryCode),
    /// Tax residency (can differ from residence)
    TaxResidency(CountryCode),
    /// Custom country type for future extensions
    Custom(Symbol, CountryCode),
}

/// Represents different types of country relationships for organizations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrganizationCountryRelation {
    /// Country of incorporation/registration
    Incorporation(CountryCode),
    /// Countries where organization operates
    OperatingJurisdiction(CountryCode),
    /// Tax jurisdiction
    TaxJurisdiction(CountryCode),
    /// Country where funds originate
    SourceOfFunds(CountryCode),
    /// Custom country type for future extensions
    Custom(Symbol, CountryCode),
}

/// Unified country relationship that can be either individual or organizational
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CountryRelation {
    Individual(IndividualCountryRelation),
    Organization(OrganizationCountryRelation),
}

/// A country data containing the country relationship and optional metadata
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryData {
    /// Type of country relationship
    pub country: CountryRelation,
    /// Optional metadata (e.g., visa type, validity period)
    pub metadata: Option<Map<Symbol, String>>,
}

/// Complete identity profile containing identity type and country data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityProfile {
    pub identity_type: IdentityType,
    pub countries: Vec<CountryData>,
}

/// Storage keys for the data associated with Identity Storage Registry.
#[contracttype]
pub enum IRSStorageKey {
    /// Maps account address to identity address
    Identity(Address),
    /// Maps an account to its complete identity profile
    IdentityProfile(Address),
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

/// Retrieves the complete identity profile for a given account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
///
/// # Errors
///
/// * [`IRSError::IdentityNotFound`] - If no identity profile is found for the
///   account.
pub fn get_identity_profile(e: &Env, account: &Address) -> IdentityProfile {
    let key = IRSStorageKey::IdentityProfile(account.clone());
    get_persistent_entry(e, &key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound))
}

/// Retrieves a specific country data entry by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
/// * `index` - The index of the country data to retrieve.
///
/// # Errors
///
/// * [`IRSError::CountryDataNotFound`] - If the index is out of bounds.
pub fn get_country_data(e: &Env, account: &Address, index: u32) -> CountryData {
    let profile = get_identity_profile(e, account);
    profile
        .countries
        .get(index)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryDataNotFound))
}

/// Retrieves all country data for a given account. Returns an empty vector if
/// not set.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to query.
pub fn get_country_data_entries(e: &Env, account: &Address) -> Vec<CountryData> {
    match get_persistent_entry::<IdentityProfile>(
        e,
        &IRSStorageKey::IdentityProfile(account.clone()),
    ) {
        Some(profile) => profile.countries,
        None => Vec::new(e),
    }
}

// ################## CHANGE STATE ##################

/// Stores a new identity with a complete identity profile.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to associate with the identity.
/// * `identity` - The identity address to store.
/// * `identity_type` - The type of identity (Individual or Organization).
/// * `initial_countries` - A vector of initial country data.
///
/// # Errors
///
/// * [`IRSError::IdentityAlreadyExists`] - If an identity is already stored for
///   the `account`.
/// * [`IRSError::EmptyCountryList`] - If `initial_countries` is empty.
/// * [`IRSError::MaxCountryEntriesReached`] - If the number of
///   `initial_countries` exceeds `MAX_COUNTRY_ENTRIES`.
/// * refer to [`validate_country_relations`] errors.
///
/// # Events
///
/// * topics - `["identity_stored", account: Address, identity: Address]`
/// * data - `[]`
///
/// Emits for each country data added:
/// * topics - `["country_added", account: Address, country_data: CountryData]`
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
    identity_type: IdentityType,
    initial_countries: &Vec<CountryData>,
) {
    if initial_countries.is_empty() {
        panic_with_error!(e, IRSError::EmptyCountryList)
    }
    if initial_countries.len() > MAX_COUNTRY_ENTRIES {
        panic_with_error!(e, IRSError::MaxCountryEntriesReached);
    }

    // Validate that country relations match the identity type
    validate_country_relations(e, &identity_type, initial_countries);

    let identity_key = IRSStorageKey::Identity(account.clone());
    if e.storage().persistent().has(&identity_key) {
        panic_with_error!(e, IRSError::IdentityAlreadyExists)
    }
    e.storage().persistent().set(&identity_key, identity);

    emit_identity_stored(e, account, identity);

    let profile = IdentityProfile { identity_type, countries: initial_countries.clone() };

    e.storage().persistent().set(&IRSStorageKey::IdentityProfile(account.clone()), &profile);

    for country_data in initial_countries.iter() {
        emit_country_data_event(e, CountryDataEvent::Added, account, &country_data);
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

/// Removes an identity and all associated country data.
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
/// Emits for each country data removed:
/// * topics - `["country_removed", account: Address, country_relation:
///   CountryRelation]`
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
    let identity_key = IRSStorageKey::Identity(account.clone());

    let identity: Address = e
        .storage()
        .persistent()
        .get(&identity_key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound));
    e.storage().persistent().remove(&identity_key);

    emit_identity_unstored(e, account, &identity);

    // Remove all associated identity profile
    let profile_key = IRSStorageKey::IdentityProfile(account.clone());
    let profile: IdentityProfile =
        e.storage().persistent().get(&profile_key).expect("identity profile must be already set");
    e.storage().persistent().remove(&profile_key);

    for country_data in profile.countries {
        emit_country_data_event(e, CountryDataEvent::Removed, account, &country_data);
    }
}

/// Adds multiple country data entries to an existing identity.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address to add country data to.
/// * `country_data_list` - A vector of country data to add.
///
/// # Errors
///
/// * [`IRSError::EmptyCountryList`] - If `country_data_list` is empty.
/// * [`IRSError::MaxCountryEntriesReached`] - If the number of country data
///   entries exceeds `MAX_COUNTRY_ENTRIES`.
/// * refer to [`validate_country_relations`] errors.
///
/// # Events
///
/// Emits for each country data added:
/// * topics - `["country_added", account: Address, country_relation:
///   CountryRelation]`
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
pub fn add_country_data_entries(e: &Env, account: &Address, country_data_list: &Vec<CountryData>) {
    if country_data_list.is_empty() {
        panic_with_error!(e, IRSError::EmptyCountryList)
    }

    let mut profile: IdentityProfile =
        get_persistent_entry(e, &IRSStorageKey::IdentityProfile(account.clone()))
            .expect("identity profile must be already set");

    // Validate that country relations match the identity type
    validate_country_relations(e, &profile.identity_type, country_data_list);

    profile.countries.append(country_data_list);
    if profile.countries.len() > MAX_COUNTRY_ENTRIES {
        panic_with_error!(e, IRSError::MaxCountryEntriesReached);
    }

    let key = IRSStorageKey::IdentityProfile(account.clone());
    e.storage().persistent().set(&key, &profile);

    for country_data in country_data_list.iter() {
        emit_country_data_event(e, CountryDataEvent::Added, account, &country_data);
    }
}

/// Modifies an existing country data entry by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address whose country data is being modified.
/// * `index` - The index of the country data to modify.
/// * `country_data` - The new country data.
///
/// # Errors
///
/// * [`IRSError::CountryDataNotFound`] - If the index is out of bounds.
/// * refer to [`validate_country_relations`] errors.
///
/// # Events
///
/// * topics - `["country_modified", account: Address, country_relation:
///   CountryRelation]`
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
pub fn modify_country_data(e: &Env, account: &Address, index: u32, country_data: &CountryData) {
    let mut profile = get_identity_profile(e, account);
    if index >= profile.countries.len() {
        panic_with_error!(e, IRSError::CountryDataNotFound);
    }

    // Validate that the new country relation matches the identity type
    validate_country_relations(e, &profile.identity_type, &vec![e, country_data.clone()]);
    profile.countries.set(index, country_data.clone());

    let key = IRSStorageKey::IdentityProfile(account.clone());
    e.storage().persistent().set(&key, &profile);

    emit_country_data_event(e, CountryDataEvent::Modified, account, country_data);
}

/// Deletes a country data entry by its index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address whose country data is being deleted.
/// * `index` - The index of the country data to delete.
///
/// # Errors
///
/// * [`IRSError::CountryDataNotFound`] - If the index is out of bounds.
/// * [`IRSError::EmptyCountryList`] - If attempting to delete the last country
///   data entry.
///
/// # Events
///
/// * topics - `["country_removed", account: Address, country_relation:
///   CountryRelation]`
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
pub fn delete_country_data(e: &Env, account: &Address, index: u32) {
    let mut profile = get_identity_profile(e, account);

    if profile.countries.len() == 1 {
        panic_with_error!(e, IRSError::EmptyCountryList)
    }

    let country_data_to_remove = profile
        .countries
        .get(index)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::CountryDataNotFound));

    profile.countries.remove(index);

    let key = IRSStorageKey::IdentityProfile(account.clone());
    e.storage().persistent().set(&key, &profile);

    emit_country_data_event(e, CountryDataEvent::Removed, account, &country_data_to_remove);
}

// ################## HELPERS ##################

/// Validates that country relations match the identity type.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity_type` - The type of identity (Individual or Organization).
/// * `country_data_list` - The list of country data to validate.
///
/// # Errors
///
/// * [`IRSError::CountryRelationMismatch`] - If any country relation doesn't
///   match the identity type.
pub fn validate_country_relations(
    e: &Env,
    identity_type: &IdentityType,
    country_data_list: &Vec<CountryData>,
) {
    for country_data in country_data_list.iter() {
        match (identity_type, &country_data.country) {
            (IdentityType::Individual, CountryRelation::Individual(_)) => {
                // Valid: Individual identity with individual country relation
            }
            (IdentityType::Organization, CountryRelation::Organization(_)) => {
                // Valid: Organization identity with organization country
                // relation
            }
            _ => {
                // Invalid: Mismatched identity type and country relation
                panic_with_error!(e, IRSError::CountryRelationMismatch);
            }
        }
    }
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
