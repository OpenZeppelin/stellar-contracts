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
//! relationship types with country codes.
//!
//! ## Flexible Country Relations
//!
//! The system supports flexible mixing of country relationship types to
//! accommodate complex regulatory requirements:
//!
//! - **Individual** identities can have both individual and organizational
//!   country relations
//! - **Organization** identities can include country data for key individuals
//!   (KYB requirements)
//!
//! This flexibility supports Know-Your-Business (KYB) processes where
//! organizations must provide jurisdictional information about:
//! - Ultimate Beneficial Owners (UBOs)
//! - Key management personnel
//! - Authorized signatories
//! - Board members and directors
//!
//! For example, a US-incorporated company may need to track:
//! - `Incorporation(840)` - Company incorporated in USA
//! - `Residence(276)` - CEO resides in Germany
//! - `Citizenship(756)` - CFO is a Swiss citizen
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
//! ## Usage Patterns
//!
//! ### Individual Identity
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
//! add_identity(&e, &account, &identity, IdentityType::Individual, &country_data);
//! ```
//!
//! ### Organization with KYB Data
//! ```rust
//! // Organization including individual data for KYB compliance
//! let country_data = vec![
//!     // Corporate data
//!     CountryData {
//!         country: CountryRelation::Organization(
//!             OrganizationCountryRelation::Incorporation(840), // USA
//!         ),
//!         metadata: Some(metadata_map!("entity_type" => "Corporation")),
//!     },
//!     CountryData {
//!         country: CountryRelation::Organization(
//!             OrganizationCountryRelation::OperatingJurisdiction(276), // Germany
//!         ),
//!         metadata: None,
//!     },
//!     // Individual data for KYB (Ultimate Beneficial Owner)
//!     CountryData {
//!         country: CountryRelation::Individual(
//!             IndividualCountryRelation::Residence(756), // Switzerland
//!         ),
//!         metadata: Some(metadata_map!("role" => "UBO", "name" => "John Doe")),
//!     },
//!     CountryData {
//!         country: CountryRelation::Individual(
//!             IndividualCountryRelation::Citizenship(250), // France
//!         ),
//!         metadata: Some(metadata_map!("role" => "CEO", "name" => "Jane Smith")),
//!     },
//! ];
//!
//! add_identity(&e, &account, &identity, IdentityType::Organization, &country_data);
//! ```
//! ## Constraints
//!
//! - Maximum 15 country data entries per identity
//! - At least one country data entry required per identity
//! - All operations require proper authorization (handled by implementer)
//! - Metadata can be used to provide additional context for mixed relation
//!   types
//!
//! ## Flexible Country Data Interface
//!
//! The trait interface uses [`Val`] for country data parameters instead of the
//! concrete [`CountryData`] type. This allows implementors to define their own
//! country data structure while preserving the same contract interface. The
//! library provides [`CountryData`] as a reference implementation with rich
//! jurisdictional modeling (see [Data Model](#data-model)), but contracts are
//! free to use any `#[contracttype]` that suits their compliance requirements.
//!
//! Implementors that use a custom country data type handle serialization and
//! deserialization between `Val` and their type internally, while the public
//! interface remains uniform across all implementations.
//!
//! An alternative design would use an associated type on the trait (e.g.,
//! `type CountryData: FromVal<Env, Val>`), but the Soroban SDK does not
//! support deriving `#[contractclient]` for traits with associated types,
//! which would prevent generating cross-contract clients from the trait
//! definition. Using `Val` directly avoids this limitation.
//!
//! ## ⚠️ Privacy and Security Considerations
//!
//! **IMPORTANT: The reference [`CountryData`] implementation stores compliance
//! data in plaintext on the blockchain, making it publicly accessible to all
//! network participants.**
//!
//! ### Public Data Exposure
//!
//! All data stored through the reference implementation, including:
//! - Identity types (Individual/Organization)
//! - Country relationships (citizenship, residence, incorporation, etc.)
//! - Associated metadata (names, roles, entity types)
//!
//! is **public and accessible** to anyone with access to the blockchain.
//!
//! ### Risks
//!
//! Storing personally identifiable information (PII) and sensitive compliance
//! data in plaintext on an immutable public ledger creates several risks:
//!
//! - **Data Harvesting**: Malicious actors can collect and aggregate sensitive
//!   user information for fraud, identity theft, or targeted attacks
//! - **Regulatory Compliance**: May violate data protection regulations (GDPR,
//!   CCPA, etc.) that require data minimization and the right to erasure
//! - **Immutability**: Once stored, data cannot be deleted or modified to
//!   comply with "right to be forgotten" requirements
//! - **Correlation Attacks**: Public data can be cross-referenced with other
//!   on-chain or off-chain data sources to de-anonymize users
//!
//! ### Privacy-Preserving Alternatives
//!
//! Because the trait accepts `Val` for country data, implementors can define
//! their own privacy-preserving types while keeping the same contract
//! interface. Below are examples of alternative country data types:
//!
//! #### 1. Hash-Based Commitments
//!
//! Store only cryptographic hashes of compliance data:
//!
//! ```rust
//! use soroban_sdk::{contracttype, BytesN, Env, FromVal, IntoVal, Val};
//!
//! #[contracttype]
//! pub struct HashCommitment {
//!     pub commitment: BytesN<32>,
//!     pub timestamp: u64,
//! }
//!
//! // Inside the trait implementation, convert between Val and the
//! // custom type:
//! fn add_identity(
//!     e: &Env,
//!     account: Address,
//!     identity: Address,
//!     country_data_list: Vec<Val>,
//!     operator: Address,
//! ) {
//!     // Convert each Val entry to the custom type
//!     let commitments: Vec<HashCommitment> =
//!         country_data_list.iter().map(|v| HashCommitment::from_val(e, &v)).collect();
//!     // ... store commitments, emit events using val.into_val(e), etc.
//! }
//! ```
//!
//! #### 2. Merkle Tree Commitments
//!
//! Store a Merkle root for selective disclosure:
//!
//! ```rust
//! use soroban_sdk::{contracttype, BytesN, Env, FromVal, IntoVal, Symbol, Val};
//!
//! #[contracttype]
//! pub struct MerkleCommitment {
//!     pub merkle_root: BytesN<32>,
//!     pub attribute_type: Symbol,
//! }
//!
//! // Inside the trait implementation:
//! fn get_country_data(e: &Env, account: Address, index: u32) -> Val {
//!     let commitment: MerkleCommitment = /* load from storage */;
//!     commitment.into_val(e)
//! }
//! ```
//!
//! #### 3. Zero-Knowledge Proofs
//!
//! Store verification keys for ZK proofs:
//!
//! ```rust
//! use soroban_sdk::{contracttype, BytesN, Env, FromVal, IntoVal, Symbol, Val};
//!
//! #[contracttype]
//! pub struct ZKCommitment {
//!     pub verification_key: BytesN<32>,
//!     pub proof_type: Symbol,
//! }
//!
//! // Inside the trait implementation:
//! fn get_country_data_entries(e: &Env, account: Address) -> Vec<Val> {
//!     let entries: Vec<ZKCommitment> = /* load from storage */;
//!     Vec::from_iter(e, entries.iter().map(|c| c.into_val(e)))
//! }
//! ```
//!
//! #### 4. Off-Chain Storage with On-Chain Attestations
//!
//! Store attestation metadata from trusted verifiers:
//!
//! ```rust
//! use soroban_sdk::{contracttype, Address, BytesN, Env, FromVal, IntoVal, Symbol, Val};
//!
//! #[contracttype]
//! pub struct ComplianceAttestation {
//!     pub attestor: Address,
//!     pub data_hash: BytesN<32>,
//!     pub attribute_type: Symbol,
//! }
//!
//! // Inside the trait implementation:
//! fn modify_country_data(
//!     e: &Env,
//!     account: Address,
//!     index: u32,
//!     country_data: Val,
//!     operator: Address,
//! ) {
//!     let attestation = ComplianceAttestation::from_val(e, &country_data);
//!     // ... validate and store, then emit event:
//!     emit_country_data_event(e, CountryDataEvent::Modified, &account, &attestation.into_val(e));
//! }
//! ```
//!
//! ### Recommendation
//!
//! The reference [`CountryData`] implementation is suitable for:
//! - Non-sensitive jurisdictional data
//! - Public compliance frameworks where transparency is required
//! - Testing and development environments
//!
//! For production deployments with sensitive data, define a custom country
//! data type using one of the privacy-preserving approaches above.
mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, Env, IntoVal, Val, Vec};
pub use storage::{
    add_country_data_entries, add_identity, delete_country_data, get_country_data,
    get_country_data_entries, get_identity_profile, get_recovered_to, modify_country_data,
    modify_identity, recover_identity, remove_identity, stored_identity, validate_country_data,
    CountryData, CountryRelation, IdentityProfile, IdentityType, IndividualCountryRelation,
    OrganizationCountryRelation,
};

use crate::rwa::utils::token_binder::TokenBinder;

/// The core trait for managing basic identities.
///
/// Country data parameters use [`Val`] to allow implementors to define their
/// own country data structure. The library provides [`CountryData`] as a
/// reference implementation, but any `#[contracttype]` can be used by
/// converting to/from `Val` internally.
#[contracttrait]
pub trait IdentityRegistryStorage: TokenBinder {
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
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`add_identity`] for the
    /// implementation.
    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        country_data_list: Vec<Val>,
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
    /// * topics - `["country_removed", account: Address]`
    /// * data - `[country_data: Val]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`remove_identity`] for the
    /// implementation.
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
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`modify_identity`] for the
    /// implementation.
    fn modify_identity(e: &Env, account: Address, identity: Address, operator: Address);

    /// Recovers an identity by transferring it from an old account to a new
    /// account.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `old_account` - The account address from which to recover the
    ///   identity.
    /// * `new_account` - The account address to which the identity will be
    ///   transferred.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["identity_recovered", old_account: Address, new_account:
    ///   Address]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`recover_identity`] for the
    /// implementation.
    fn recover_identity(e: &Env, old_account: Address, new_account: Address, operator: Address);

    /// Retrieves the stored identity for a given account.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    fn stored_identity(e: &Env, account: Address) -> Address {
        storage::stored_identity(e, &account)
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
    fn get_recovered_to(e: &Env, old_account: Address) -> Option<Address> {
        storage::get_recovered_to(e, &old_account)
    }
}

/// Trait for managing multiple country data entries associated with an
/// identity.
///
/// Like [`IdentityRegistryStorage`], country data parameters use [`Val`]
/// for flexibility. Default implementations convert between the reference
/// [`CountryData`] type and `Val`.
#[contracttrait]
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
    /// * topics - `["country_added", account: Address]`
    /// * data - `[country_data: Val]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`add_country_data_entries`] for the implementation.
    fn add_country_data_entries(
        e: &Env,
        account: Address,
        country_data_list: Vec<Val>,
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
    /// * topics - `["country_modified", account: Address]`
    /// * data - `[country_data: Val]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`modify_country_data`] for the
    /// implementation.
    fn modify_country_data(
        e: &Env,
        account: Address,
        index: u32,
        country_data: Val,
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
    /// * topics - `["country_removed", account: Address]`
    /// * data - `[country_data: Val]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`delete_country_data`] for the
    /// implementation.
    fn delete_country_data(e: &Env, account: Address, index: u32, operator: Address);

    /// Retrieves all country data entries for a given account.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Val> {
        Vec::from_iter(
            e,
            get_country_data_entries(e, &account).iter().map(|entry| entry.into_val(e)),
        )
    }

    /// Retrieves a specific country data entry by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    /// * `index` - The index of the country data to retrieve.
    fn get_country_data(e: &Env, account: Address, index: u32) -> Val {
        storage::get_country_data(e, &account, index).into_val(e)
    }
}

// ################## ERRORS ##################

/// Error codes for the Identity Registry Storage system.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IRSError {
    /// An identity already exists for the given account.
    IdentityOverwrite = 320,
    /// No identity found for the given account.
    IdentityNotFound = 321,
    /// Country data not found at the specified index.
    CountryDataNotFound = 322,
    /// Identity can't be with empty country data list.
    EmptyCountryList = 323,
    /// The maximum number of country entries has been reached.
    MaxCountryEntriesReached = 324,
    /// Account has been recovered and cannot be used.
    AccountRecovered = 325,
    /// Metadata has too many entries (exceeds MAX_METADATA_ENTRIES).
    MetadataTooManyEntries = 326,
    /// Metadata string value is too long (exceeds MAX_METADATA_STRING_LEN).
    MetadataStringTooLong = 327,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const IDENTITY_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const IDENTITY_TTL_THRESHOLD: u32 = IDENTITY_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// The maximum number of country data entries that can be associated with a
/// single identity.
pub const MAX_COUNTRY_ENTRIES: u32 = 15;

/// The maximum number of metadata entries per CountryData.
pub const MAX_METADATA_ENTRIES: u32 = 10;

/// The maximum length of a metadata string value.
pub const MAX_METADATA_STRING_LEN: u32 = 100;

// ################## EVENTS ##################

pub enum CountryDataEvent {
    Added,
    Removed,
    Modified,
}

/// Event emitted when an identity is stored for an account.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityStored {
    #[topic]
    pub account: Address,
    #[topic]
    pub identity: Address,
}

/// Emits an event when an identity is stored for an account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address associated with the identity.
/// * `identity` - The identity address that was stored.
pub fn emit_identity_stored(e: &Env, account: &Address, identity: &Address) {
    IdentityStored { account: account.clone(), identity: identity.clone() }.publish(e);
}

/// Event emitted when an identity is removed from an account.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityUnstored {
    #[topic]
    pub account: Address,
    #[topic]
    pub identity: Address,
}

/// Emits an event when an identity is removed from an account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `account` - The account address that had its identity removed.
/// * `identity` - The identity address that was removed.
pub fn emit_identity_unstored(e: &Env, account: &Address, identity: &Address) {
    IdentityUnstored { account: account.clone(), identity: identity.clone() }.publish(e);
}

/// Event emitted when an identity is modified for an account.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityModified {
    #[topic]
    pub old_identity: Address,
    #[topic]
    pub new_identity: Address,
}

/// Emits an event when an identity is modified for an account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `old_identity` - The previous identity address.
/// * `new_identity` - The new identity address.
pub fn emit_identity_modified(e: &Env, old_identity: &Address, new_identity: &Address) {
    IdentityModified { old_identity: old_identity.clone(), new_identity: new_identity.clone() }
        .publish(e);
}

/// Event emitted when an identity is recovered for a new account.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityRecovered {
    #[topic]
    pub old_account: Address,
    #[topic]
    pub new_account: Address,
}

/// Emits an event when an identity is recovered for a new account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `old_account` - The previous account address.
/// * `new_account` - The new account address.
pub fn emit_identity_recovered(e: &Env, old_account: &Address, new_account: &Address) {
    IdentityRecovered { old_account: old_account.clone(), new_account: new_account.clone() }
        .publish(e);
}

/// Event emitted for country data operations.
#[contractevent]
#[derive(Clone, Debug)]
pub struct CountryDataAdded {
    #[topic]
    pub account: Address,
    pub country_data: Val,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CountryDataRemoved {
    #[topic]
    pub account: Address,
    pub country_data: Val,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CountryDataModified {
    #[topic]
    pub account: Address,
    pub country_data: Val,
}

/// Emits an event for country data operations (add, remove, modify).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `event_type` - The type of country data event.
/// * `account` - The account address associated with the country data.
/// * `country_data` - The country data that was affected.
pub fn emit_country_data_event(
    e: &Env,
    event_type: CountryDataEvent,
    account: &Address,
    country_data: &Val,
) {
    match event_type {
        CountryDataEvent::Added => {
            CountryDataAdded { account: account.clone(), country_data: *country_data }.publish(e)
        }
        CountryDataEvent::Removed => {
            CountryDataRemoved { account: account.clone(), country_data: *country_data }.publish(e)
        }
        CountryDataEvent::Modified => {
            CountryDataModified { account: account.clone(), country_data: *country_data }.publish(e)
        }
    }
}
