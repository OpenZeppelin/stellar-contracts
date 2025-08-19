use soroban_sdk::{contract, contractimpl, Address, Env, Vec};
use stellar_tokens::rwa::{
    identity_storage_registry::{
        self as identity_storage, CountryProfile, CountryProfileManager, IdentityRegistryStorage,
    },
    utils::token_binder::{self as binder, TokenBinder},
};

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    pub fn __constructor(_e: Env, _admin: Address) {}

    pub fn bind_tokens(e: &Env, tokens: Vec<Address>, operator: Address) {
        // TODO: access control operator
        operator.require_auth();
        binder::bind_tokens(e, &tokens);
    }

    pub fn get_token_index(e: &Env, token: Address) -> u32 {
        binder::get_token_index(e, &token)
    }
}

#[contractimpl]
impl TokenBinder for IdentityRegistryContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        binder::linked_tokens(e)
    }

    fn bind_token(e: &Env, token: Address, operator: Address) {
        // TODO: access control operator
        operator.require_auth();
        binder::bind_token(e, &token);
    }

    fn unbind_token(e: &Env, token: Address, operator: Address) {
        // TODO: access control operator
        operator.require_auth();
        binder::unbind_token(e, &token);
    }
}

#[contractimpl]
impl IdentityRegistryStorage for IdentityRegistryContract {
    type CountryProfile = CountryProfile;

    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        initial_profiles: Vec<CountryProfile>,
        _operator: Address,
    ) {
        identity_storage::add_identity(e, &account, &identity, &initial_profiles);
    }

    fn modify_identity(e: &Env, account: Address, new_identity: Address, _operator: Address) {
        identity_storage::modify_identity(e, &account, &new_identity);
    }

    fn remove_identity(e: &Env, account: Address, _operator: Address) {
        identity_storage::remove_identity(e, &account);
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        identity_storage::get_identity(e, &account)
    }
}

#[contractimpl]
impl CountryProfileManager for IdentityRegistryContract {
    fn add_country_profiles(
        e: &Env,
        account: Address,
        profiles: Vec<CountryProfile>,
        _operator: Address,
    ) {
        identity_storage::add_country_profiles(e, &account, &profiles);
    }

    fn modify_country_profile(
        e: &Env,
        account: Address,
        index: u32,
        profile: CountryProfile,
        _operator: Address,
    ) {
        identity_storage::modify_country_profile(e, &account, index, &profile);
    }

    fn delete_country_profile(e: &Env, account: Address, index: u32, _operator: Address) {
        identity_storage::delete_country_profile(e, &account, index);
    }

    fn get_country_profile(e: &Env, account: Address, index: u32) -> CountryProfile {
        identity_storage::get_country_profile(e, &account, index)
    }

    fn get_country_profiles(e: &Env, account: Address) -> Vec<CountryProfile> {
        identity_storage::get_country_profiles(e, &account)
    }
}
