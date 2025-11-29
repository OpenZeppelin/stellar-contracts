/// ## How Country Data Work
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
/// - `Custom(Symbol::new(e, "Subsidiary"), 792)` - Custom subsidiary location:
///   Turkey
///
/// ### Flexible Country Relations
///
/// This flexible structure allows an account to hold multiple country
/// relationships and supports mixing of individual and organizational country
/// relations to accommodate Know-Your-Business (KYB) requirements.
///
/// **Examples of mixed relations:**
/// - Organizations can include individual country data for Ultimate Beneficial
///   Owners (UBOs), key management personnel, or authorized signatories
/// - Individual identities can include organizational relationships when
///   relevant for compliance
///
/// **KYB Example for a US Corporation:**
/// ```rust
/// let kyb_country_data = vec![
///     // Corporate data
///     CountryData {
///         country: CountryRelation::Organization(
///             OrganizationCountryRelation::Incorporation(840) // USA
///         ),
///         metadata: Some(metadata_map!("entity_type" => "Corporation")),
///     },
///     // UBO individual data
///     CountryData {
///         country: CountryRelation::Individual(
///             IndividualCountryRelation::Residence(276) // Germany
///         ),
///         metadata: Some(metadata_map!("role" => "UBO", "name" => "John Doe")),
///     },
///     CountryData {
///         country: CountryRelation::Individual(
///             IndividualCountryRelation::Citizenship(756) // Switzerland
///         ),
///         metadata: Some(metadata_map!("role" => "CEO", "ownership" => "25%")),
///     },
/// ];
/// ```
///
/// When a new identity is registered for an account, it must be created with at
/// least one initial country data. Afterward, more country data can be added
/// (up to MAX_COUNTRY_ENTRIES), modified, or removed as needed.
///
/// ### Design Principles
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
/// 4. **Flexible Relation Types**: Country data entries can mix individual and
///    organizational relations within the same identity to support complex
///    regulatory requirements like KYB processes.
/// 5. **Metadata Context**: Use the optional metadata field to provide context
///    for mixed relation types (e.g., role, name, ownership percentage).
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
///         identity_registry_storage::add_country_data_entries(e, &account, &country_entries);
///     }
///     // other methods
/// }
/// ```
///
/// ## Account Recovery
///
/// The system supports account recovery for lost or compromised wallets while
/// maintaining strict security and audit trail requirements:
///
/// - **Recovered accounts cannot have new identities added**: Once an account
///   has been recovered, it is permanently marked and cannot be reused for new
///   identities.
/// - **Cannot recover to an already-recovered account**: An account that was
///   previously used as a recovery target cannot be used again.
/// - **Proper sequencing enforced**: The system enforces the correct recovery
///   sequence: `recover_identity` must be called before `recovery_balance` to
///   ensure identity verification precedes asset transfer.
use soroban_sdk::{
    contracttype, panic_with_error, Address, Env, Map, String, Symbol, TryFromVal, Val, Vec,
};

use crate::rwa::identity_registry_storage::{
    CountryDataEvent, IRSError, IDENTITY_EXTEND_AMOUNT,
    IDENTITY_TTL_THRESHOLD, MAX_COUNTRY_ENTRIES,
};

#[cfg(not(feature = "certora"))]
use crate::rwa::identity_registry_storage::{
    emit_country_data_event, emit_identity_modified, emit_identity_recovered, emit_identity_stored,
    emit_identity_unstored,
};

/// Represents the type of identity holder
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityType {
    Individual,
    Organization,
}

/// Represents different types of country relationships for individuals
/// ISO 3166-1 numeric country code
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndividualCountryRelation {
    /// Country of residence
    Residence(u32),
    /// Country of citizenship
    Citizenship(u32),
    /// Country where funds originate
    SourceOfFunds(u32),
    /// Tax residency (can differ from residence)
    TaxResidency(u32),
    /// Custom country type for future extensions
    Custom(Symbol, u32),
}

/// Represents different types of country relationships for organizations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrganizationCountryRelation {
    /// Country of incorporation/registration
    Incorporation(u32),
    /// Countries where organization operates
    OperatingJurisdiction(u32),
    /// Tax jurisdiction
    TaxJurisdiction(u32),
    /// Country where funds originate
    SourceOfFunds(u32),
    /// Custom country type for future extensions
    Custom(Symbol, u32),
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
    /// Maps old account to new account after recovery
    RecoveredTo(Address),
}

// ################## QUERY STATE ##################

/// Retrieves the stored identity for a given account.
///
/// Each user account interacting with the RWA token must be linked to an
/// identity contract that stores compliance-related data and other regulatory
/// information.
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
pub fn stored_identity(e: &Env, account: &Address) -> Address {
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

/// Retrieves the recovery target address for a recovered account.
///
/// Returns `Some(new_account)` if the account has been recovered to a new
/// account, or `None` if the account has not been recovered.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `old_account` - The old account address to check.
pub fn get_recovered_to(e: &Env, old_account: &Address) -> Option<Address> {
    get_persistent_entry(e, &IRSStorageKey::RecoveredTo(old_account.clone()))
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
/// * [`IRSError::AccountRecovered`] - If the `account` has been recovered to
///   another account.
/// * [`IRSError::IdentityOverwrite`] - If an identity is already stored for the
///   `account`.
/// * [`IRSError::EmptyCountryList`] - If `initial_countries` is empty.
/// * [`IRSError::MaxCountryEntriesReached`] - If the number of
///   `initial_countries` exceeds `MAX_COUNTRY_ENTRIES`.
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
    // Check if account has been recovered
    if get_recovered_to(e, account).is_some() {
        panic_with_error!(e, IRSError::AccountRecovered)
    }

    if initial_countries.is_empty() {
        panic_with_error!(e, IRSError::EmptyCountryList)
    }
    if initial_countries.len() > MAX_COUNTRY_ENTRIES {
        panic_with_error!(e, IRSError::MaxCountryEntriesReached);
    }

    let identity_key = IRSStorageKey::Identity(account.clone());
    if e.storage().persistent().has(&identity_key) {
        panic_with_error!(e, IRSError::IdentityOverwrite)
    }
    e.storage().persistent().set(&identity_key, identity);
    #[cfg(not(feature = "certora"))]
    emit_identity_stored(e, account, identity);

    let profile = IdentityProfile { identity_type, countries: initial_countries.clone() };

    e.storage().persistent().set(&IRSStorageKey::IdentityProfile(account.clone()), &profile);

    for country_data in initial_countries.iter() {
        #[cfg(not(feature = "certora"))]
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
    #[cfg(not(feature = "certora"))]
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
    #[cfg(not(feature = "certora"))]
    emit_identity_unstored(e, account, &identity);

    // Remove all associated identity profile
    let profile_key = IRSStorageKey::IdentityProfile(account.clone());
    let profile: IdentityProfile =
        e.storage().persistent().get(&profile_key).expect("identity profile must be already set");
    e.storage().persistent().remove(&profile_key);

    for country_data in profile.countries {
        #[cfg(not(feature = "certora"))]
        emit_country_data_event(e, CountryDataEvent::Removed, account, &country_data);
    }
}

/// Recovers an identity by transferring it from an old account to a new
/// account.
///
/// This function is typically used in account recovery scenarios where a user
/// needs to transfer their identity and all associated data (including country
/// data) from a compromised or lost account to a new account address.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `old_account` - The account address from which to recover the identity.
/// * `new_account` - The account address to which the identity will be
///   transferred.
///
/// # Errors
///
/// * [`IRSError::IdentityNotFound`] - If no identity is found for the
///   `old_account`.
/// * [`IRSError::AccountRecovered`] - If the `new_account` has already been
///   recovered to another account.
/// * [`IRSError::IdentityOverwrite`] - If the `new_account` is already linked
///   to an identity.
///
/// # Events
///
/// * topics - `["identity_recovered", old_account: Address, new_account:
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
pub fn recover_identity(e: &Env, old_account: &Address, new_account: &Address) {
    // Check if new_account has been recovered
    if get_recovered_to(e, new_account).is_some() {
        panic_with_error!(e, IRSError::AccountRecovered)
    }

    // Recover identity
    let old_identity_key = IRSStorageKey::Identity(old_account.clone());
    let new_identity_key = IRSStorageKey::Identity(new_account.clone());

    let identity: Address = e
        .storage()
        .persistent()
        .get(&old_identity_key)
        .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound));

    // Check if new_account is not already linked to another identity
    if e.storage().persistent().has(&new_identity_key) {
        panic_with_error!(e, IRSError::IdentityOverwrite)
    }

    e.storage().persistent().set(&new_identity_key, &identity);
    e.storage().persistent().remove(&old_identity_key);

    // Recover identity profile
    let old_profile_key = IRSStorageKey::IdentityProfile(old_account.clone());
    let new_profile_key = IRSStorageKey::IdentityProfile(new_account.clone());

    let profile: IdentityProfile = e
        .storage()
        .persistent()
        .get(&old_profile_key)
        // it would've panicked above if no IdentityProfile
        .expect("identity profile must be already set");

    e.storage().persistent().set(&new_profile_key, &profile);
    e.storage().persistent().remove(&old_profile_key);

    // Mark old account as recovered to new account
    e.storage().persistent().set(&IRSStorageKey::RecoveredTo(old_account.clone()), new_account);
    #[cfg(not(feature = "certora"))]
    emit_identity_recovered(e, old_account, new_account);
}

/// Adds multiple country data entries to an existing identity.
///
/// Does not check for duplicate country data entries.
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
            .unwrap_or_else(|| panic_with_error!(e, IRSError::IdentityNotFound));

    profile.countries.append(country_data_list);
    if profile.countries.len() > MAX_COUNTRY_ENTRIES {
        panic_with_error!(e, IRSError::MaxCountryEntriesReached);
    }

    let key = IRSStorageKey::IdentityProfile(account.clone());
    e.storage().persistent().set(&key, &profile);

    for country_data in country_data_list.iter() {
        #[cfg(not(feature = "certora"))]
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

    profile.countries.set(index, country_data.clone());

    let key = IRSStorageKey::IdentityProfile(account.clone());
    e.storage().persistent().set(&key, &profile);
    #[cfg(not(feature = "certora"))]
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
    #[cfg(not(feature = "certora"))]
    emit_country_data_event(e, CountryDataEvent::Removed, account, &country_data_to_remove);
}

// ################## HELPERS ##################

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
