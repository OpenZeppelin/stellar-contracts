//! # Identity Registry Storage Module
//!
//! This module provides a comprehensive storage system for managing identity
//! profiles and their associated country data in a Real World Assets (RWA)
//! context. It supports both individual and organizational identities with
//! type-safe country relationship management.
//!
//! ## Overview
//!
//! Each user account interacting with RWA tokens must be linked to an identity
//! contract that stores compliance-related data and other regulatory
//! information. The Identity Registry Storage system is designed to handle
//! those together with complex jurisdictional relationships for accounts.
//! Instead of simple country codes, it uses a sophisticated model that pairs
//! relationship types with country codes, ensuring data integrity through type
//! matching.
//!
//! ## Core Components
//!
//! ### Identity Types
//!
//! - **Individual**: Natural persons with personal jurisdictional ties
//! - **Organization**: Legal entities with corporate jurisdictional ties
//!
//! ### Country Relations
//!
//! **For Individuals:**
//! - `Residence(country_code)` - Country of residence
//! - `Citizenship(country_code)` - Country of citizenship
//! - `SourceOfFunds(country_code)` - Source of funds origin
//! - `TaxResidency(country_code)` - Tax residency jurisdiction
//! - `Custom(symbol, country_code)` - Custom relationship types
//!
//! **For Organizations:**
//! - `Incorporation(country_code)` - Country of incorporation/registration
//! - `OperatingJurisdiction(country_code)` - Operating jurisdiction
//! - `TaxJurisdiction(country_code)` - Tax jurisdiction
//! - `SourceOfFunds(country_code)` - Source of funds origin
//! - `Custom(symbol, country_code)` - Custom relationship types
//!
//! ## Data Model
//!
//! ```rust
//! // Identity profile containing type and country data
//! pub struct IdentityProfile {
//!     pub identity_type: IdentityType,
//!     pub countries: Vec<CountryData>,
//! }
//!
//! // Individual country data entry
//! pub struct CountryData {
//!     pub country: CountryRelation,
//!     pub metadata: Option<Map<Symbol, String>>,
//! }
//! ```
//!
//! ## Type Safety and Validation
//!
//! The system enforces strict type matching between identity types and country
//! relations:
//!
//! - **Individual** identities can only have `CountryRelation::Individual`
//!   entries
//! - **Organization** identities can only have `CountryRelation::Organization`
//!   entries
//!
//! This prevents logical inconsistencies like individuals having incorporation
//! countries or organizations having citizenship.
//!
//! ## Usage Patterns
//!
//! ### Creating an Identity
//! ```rust
//! // Individual with residence and citizenship
//! let country_data = vec![
//!     CountryData {
//!         country: CountryRelation::Individual(
//!             IndividualCountryRelation::Residence(840), // USA
//!         ),
//!         metadata: None,
//!     },
//!     CountryData {
//!         country: CountryRelation::Individual(
//!             IndividualCountryRelation::Citizenship(276), // Germany
//!         ),
//!         metadata: None,
//!     },
//! ];
//!
//! add_identity(&env, &account, &identity, IdentityType::Individual, &country_data);
//! ```
//!
//! ### Organization Example
//! ```rust
//! // Organization with incorporation and operating jurisdictions
//! let country_data = vec![
//!     CountryData {
//!         country: CountryRelation::Organization(
//!             OrganizationCountryRelation::Incorporation(840), // USA
//!         ),
//!         metadata: None,
//!     },
//!     CountryData {
//!         country: CountryRelation::Organization(
//!             OrganizationCountryRelation::OperatingJurisdiction(276), // Germany
//!         ),
//!         metadata: None,
//!     },
//! ];
//!
//! add_identity(&env, &account, &identity, IdentityType::Organization, &country_data);
//! ```
//! ## Constraints
//!
//! - Maximum 15 country data entries per identity
//! - At least one country data entry required per identity
//! - Country relations must match identity type

mod storage;
mod test;

use soroban_sdk::{contracterror, Address, Env, FromVal, Symbol, Val, Vec};
pub use storage::{
    add_country_data_entries, add_identity, delete_country_data, get_country_data,
    get_country_data_entries, get_identity, modify_country_data, modify_identity, remove_identity,
    validate_country_relations, CountryData, CountryRelation, IdentityProfile, IdentityType,
    IndividualCountryRelation, OrganizationCountryRelation,
};

use crate::rwa::utils::token_binder::TokenBinder;

/// The core trait for managing basic identities. It is generic over a
/// `CountryData` type, allowing implementers to define what constitutes a
/// country data entry.
pub trait IdentityRegistryStorage: TokenBinder {
    /// The specific type used for country data in this implementation.
    type CountryData: FromVal<Env, Val>;

    /// Stores a new identity with a set of country data entries.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to associate with the identity.
    /// * `identity` - The identity address to store.
    /// * `country_data_list` - A vector of initial country data entries.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["identity_stored", account: Address, identity: Address]`
    /// * data - `[]`
    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        country_data_list: Vec<Self::CountryData>,
        operator: Address,
    );

    /// Removes an identity and all associated country data entries.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address whose identity is being removed.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["identity_unstored", account: Address, identity: Address]`
    /// * data - `[]`
    ///
    /// Emits for each country data removed:
    /// * topics - `["country_removed", account: Address, country_data:
    ///   CountryData]`
    /// * data - `[]`
    fn remove_identity(e: &Env, account: Address, operator: Address);

    /// Modifies an existing identity.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address whose identity is being modified.
    /// * `new_identity` - The new identity address.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["identity_modified", old_identity: Address, new_identity:
    ///   Address]`
    /// * data - `[]`
    fn modify_identity(e: &Env, account: Address, identity: Address, operator: Address);

    /// Retrieves the stored identity for a given account.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    fn stored_identity(e: &Env, account: Address) -> Address;
}

/// Trait for managing multiple country data entries associated with an
/// identity.
pub trait CountryDataManager: IdentityRegistryStorage {
    /// Adds multiple country data entries to an existing identity.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to add data entries to.
    /// * `country_data_list` - A vector of country data entries to add.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// Emits for each country data entry added:
    /// * topics - `["country_added", account: Address, country_data:
    ///   CountryData]`
    /// * data - `[]`
    fn add_country_data_entries(
        e: &Env,
        account: Address,
        country_data_list: Vec<Self::CountryData>,
        operator: Address,
    );

    /// Modifies an existing country data entry by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address whose country data is being modified.
    /// * `index` - The index of the country data entry to modify.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["country_modified", account: Address, country_data:
    ///   CountryData]`
    /// * data - `[]`
    fn modify_country_data(
        e: &Env,
        account: Address,
        index: u32,
        country_data: Self::CountryData,
        operator: Address,
    );

    /// Deletes a country data entry by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address whose country data entry is being
    ///   deleted.
    /// * `index` - The index of the country data to delete.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["country_removed", account: Address, country_data:
    ///   CountryData]`
    /// * data - `[]`
    fn delete_country_data(e: &Env, account: Address, index: u32, operator: Address);

    /// Retrieves all country data entries for a given account.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Self::CountryData>;

    /// Retrieves a specific country data entry by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    /// * `index` - The index of the country data to retrieve.
    fn get_country_data(e: &Env, account: Address, index: u32) -> Self::CountryData;
}

// ################## ERRORS ##################

/// Error codes for the Identity Registry Storage system.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IRSError {
    /// An identity already exists for the given account.
    IdentityAlreadyExists = 320,
    /// No identity found for the given account.
    IdentityNotFound = 321,
    /// Country data not found at the specified index.
    CountryDataNotFound = 322,
    /// Identity can't be with empty country data list.
    EmptyCountryList = 323,
    /// The maximum number of country entries has been reached.
    MaxCountryEntriesReached = 324,
    /// Country relation type doesn't match the identity type.
    CountryRelationMismatch = 325,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const IDENTITY_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const IDENTITY_TTL_THRESHOLD: u32 = IDENTITY_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// The maximum number of country data entries that can be associated with a
/// single identity.
pub const MAX_COUNTRY_ENTRIES: u32 = 15;

// ################## EVENTS ##################

pub enum CountryDataEvent {
    Added,
    Removed,
    Modified,
}

/// Emits an event when an identity is stored for an account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address associated with the identity.
/// * `identity` - The identity address that was stored.
///
/// # Events
///
/// * topics - `["identity_stored", account: Address, identity: Address]`
/// * data - `()`
pub fn emit_identity_stored(e: &Env, account: &Address, identity: &Address) {
    let topics = (Symbol::new(e, "identity_stored"), account, identity);
    e.events().publish(topics, ());
}

/// Emits an event when an identity is removed from an account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address that had its identity removed.
/// * `identity` - The identity address that was removed.
///
/// # Events
///
/// * topics - `["identity_unstored", account: Address, identity: Address]`
/// * data - `()`
pub fn emit_identity_unstored(e: &Env, account: &Address, identity: &Address) {
    e.events().publish((Symbol::new(e, "identity_unstored"), account, identity), ());
}

/// Emits an event when an identity is modified for an account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `old_identity` - The previous identity address.
/// * `new_identity` - The new identity address.
///
/// # Events
///
/// * topics - `["identity_modified", old_identity: Address, new_identity:
///   Address]`
/// * data - `()`
pub fn emit_identity_modified(e: &Env, old_identity: &Address, new_identity: &Address) {
    let topics = (Symbol::new(e, "identity_modified"), old_identity, new_identity);
    e.events().publish(topics, ());
}

/// Emits an event for country data operations (add, remove, modify).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `event_type` - The type of country data event.
/// * `account` - The account address associated with the country data.
/// * `country_data` - The country data that was affected.
///
/// # Events
///
/// * topics - `[event_name: Symbol, account: Address, country_data:
///   CountryData]`
/// * data - `()`
///
/// Where `event_name` is one of:
/// - `"country_added"` for [`CountryDataEvent::Added`]
/// - `"country_removed"` for [`CountryDataEvent::Removed`]
/// - `"country_modified"` for [`CountryDataEvent::Modified`]
pub fn emit_country_data_event(
    e: &Env,
    event_type: CountryDataEvent,
    account: &Address,
    country_data: &CountryData,
) {
    let name = match event_type {
        CountryDataEvent::Added => Symbol::new(e, "country_added"),
        CountryDataEvent::Removed => Symbol::new(e, "country_removed"),
        CountryDataEvent::Modified => Symbol::new(e, "country_modified"),
    };

    let topics = (name, account, country_data.clone());
    e.events().publish(topics, ());
}
