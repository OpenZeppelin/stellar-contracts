use soroban_sdk::{contracterror, Address, Env, FromVal, Val, Vec};

mod storage;

use crate::rwa::utils::token_binder::TokenBinder;

/// The core trait for managing basic identities.
/// It is generic over a `CountryProfile` type, allowing implementers to define
/// what constitutes a country profile.
pub trait IdentityRegistryStorage: TokenBinder {
    /// The specific type used for country profiles in this implementation.
    type CountryProfile: FromVal<Env, Val>;

    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        country_profile: Self::CountryProfile,
        operator: Address,
    );

    fn remove_identity(e: &Env, account: Address, operator: Address);

    fn modify_identity(e: &Env, account: Address, identity: Address, operator: Address);

    fn stored_identity(e: &Env, account: Address) -> Address;
}

/// Trait for managing multiple country profiles associated with an identity.
pub trait CountryProfileManager: IdentityRegistryStorage {
    /// Adds a new country profile to an account.
    fn add_country_profile(
        e: &Env,
        account: Address,
        country_profile: Self::CountryProfile,
        operator: Address,
    );

    /// Modifies an existing country profile at a specific index.
    fn modify_country_profile(
        e: &Env,
        account: Address,
        index: u32,
        country_profile: Self::CountryProfile,
        operator: Address,
    );

    /// Deletes a country profile by its index.
    fn delete_country_profile(e: &Env, account: Address, index: u32, operator: Address);

    /// Retrieves all country profiles for an account.
    fn get_country_profiles(e: &Env, account: Address) -> Vec<Self::CountryProfile>;

    /// Retrieves the total number of country profiles for an account.
    fn get_country_profile_count(e: &Env, account: Address) -> u32;

    /// Retrieves a specific country profile by its index.
    fn get_country_profile(e: &Env, account: Address, index: u32) -> Self::CountryProfile;
}

// TODO: correct enumeration and move up to higher level
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IRSError {
    IdentityAlreadyExists = 1,
    IdentityNotFound = 2,
    CountryProfileNotFound = 3,
    NoCountryProfileLeft = 4,
}

// TODO: export one by one
pub use storage::*;

mod test;
