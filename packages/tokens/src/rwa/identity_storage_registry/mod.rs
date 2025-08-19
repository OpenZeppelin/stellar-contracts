mod storage;
mod test;

use soroban_sdk::{contracterror, Address, Env, FromVal, Symbol, Val, Vec};
pub use storage::{
    add_country_profiles, add_identity, delete_country_profile, get_country_profile,
    get_country_profiles, get_identity, modify_country_profile, modify_identity, remove_identity,
    CountryProfile,
};

use crate::rwa::utils::token_binder::TokenBinder;

/// The core trait for managing basic identities. It is generic over a
/// `CountryProfile` type, allowing implementers to define what constitutes a
/// country profile.
pub trait IdentityRegistryStorage: TokenBinder {
    /// The specific type used for country profiles in this implementation.
    type CountryProfile: FromVal<Env, Val>;

    /// Stores a new identity with a set of country profiles.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to associate with the identity.
    /// * `identity` - The identity address to store.
    /// * `initial_profiles` - A vector of initial country profiles.
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
        initial_profiles: Vec<Self::CountryProfile>,
        operator: Address,
    );

    /// Removes an identity and all associated country profiles.
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
    /// Emits for each country profile removed:
    /// * topics - `["country_removed", account: Address, profile:
    ///   CountryProfile]`
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

/// Trait for managing multiple country profiles associated with an identity.
pub trait CountryProfileManager: IdentityRegistryStorage {
    /// Adds multiple country profiles to an existing identity.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to add profiles to.
    /// * `profiles` - A vector of country profiles to add.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// Emits for each profile added:
    /// * topics - `["country_added", account: Address, profile:
    ///   CountryProfile]`
    /// * data - `[]`
    fn add_country_profiles(
        e: &Env,
        account: Address,
        country_profiles: Vec<Self::CountryProfile>,
        operator: Address,
    );

    /// Modifies an existing country profile by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address whose profile is being modified.
    /// * `index` - The index of the profile to modify.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["country_modified", account: Address, profile:
    ///   CountryProfile]`
    /// * data - `[]`
    fn modify_country_profile(
        e: &Env,
        account: Address,
        index: u32,
        country_profile: Self::CountryProfile,
        operator: Address,
    );

    /// Deletes a country profile by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address whose profile is being deleted.
    /// * `index` - The index of the profile to delete.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["country_removed", account: Address, profile:
    ///   CountryProfile]`
    /// * data - `[]`
    fn delete_country_profile(e: &Env, account: Address, index: u32, operator: Address);

    /// Retrieves all country profiles for a given account.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    fn get_country_profiles(e: &Env, account: Address) -> Vec<Self::CountryProfile>;

    /// Retrieves a specific country profile by its index.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `account` - The account address to query.
    /// * `index` - The index of the country profile to retrieve.
    fn get_country_profile(e: &Env, account: Address, index: u32) -> Self::CountryProfile;
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
    /// Country profile not found at the specified index.
    CountryProfileNotFound = 322,
    /// Identity can't be with empty country profiles list.
    EmptyCountryProfiles = 323,
    /// The maximum number of country profiles has been reached.
    MaxCountryProfilesReached = 324,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const IDENTITY_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const IDENTITY_TTL_THRESHOLD: u32 = IDENTITY_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// The maximum number of country profiles that can be associated with a single
/// identity.
pub const MAX_COUNTRY_PROFILES: u32 = 15;

// ################## EVENTS ##################

pub enum CountryProfileEvent {
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

/// Emits an event for country profile operations (add, remove, modify).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `event_type` - The type of country profile event.
/// * `account` - The account address associated with the profile.
/// * `profile` - The country profile that was affected.
///
/// # Events
///
/// * topics - `[event_name: Symbol, account: Address, profile: CountryProfile]`
/// * data - `()`
///
/// Where `event_name` is one of:
/// - `"country_added"` for [`CountryProfileEvent::Added`]
/// - `"country_removed"` for [`CountryProfileEvent::Removed`]
/// - `"country_modified"` for [`CountryProfileEvent::Modified`]
pub fn emit_country_profile_event(
    e: &Env,
    event_type: CountryProfileEvent,
    account: &Address,
    profile: &CountryProfile,
) {
    let name = match event_type {
        CountryProfileEvent::Added => Symbol::new(e, "country_added"),
        CountryProfileEvent::Removed => Symbol::new(e, "country_removed"),
        CountryProfileEvent::Modified => Symbol::new(e, "country_modified"),
    };

    let topics = (name, account, profile.clone());
    e.events().publish(topics, ());
}
