use soroban_sdk::{contract, contractimpl, Address, Env, Vec};
use stellar_tokens::rwa::{
    identity_storage_registry::{
        self as irs, CountryProfile, CountryProfileManager, IdentityRegistryStorage,
    },
    utils::token_binder::TokenBinder,
};

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    pub fn __constructor(_e: Env, _admin: Address) {}
}

#[contractimpl]
impl TokenBinder for IdentityRegistryContract {
    fn linked_tokens(_e: &Env) -> Vec<Address> {
        unimplemented!()
    }

    fn bind_token(_e: &Env, _token: Address, _operator: Address) {
        unimplemented!()
    }

    fn unbind_token(_e: &Env, _token: Address, _operator: Address) {
        unimplemented!()
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
        irs::add_identity(e, &account, &identity, &initial_profiles);
    }

    fn modify_identity(e: &Env, account: Address, new_identity: Address, _operator: Address) {
        irs::modify_identity(e, &account, &new_identity);
    }

    fn remove_identity(e: &Env, account: Address, _operator: Address) {
        irs::remove_identity(e, &account);
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        irs::get_identity(e, &account)
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
        irs::add_country_profiles(e, &account, &profiles);
    }

    fn modify_country_profile(
        e: &Env,
        account: Address,
        index: u32,
        profile: CountryProfile,
        _operator: Address,
    ) {
        irs::modify_country_profile(e, &account, index, &profile);
    }

    fn delete_country_profile(e: &Env, account: Address, index: u32, _operator: Address) {
        irs::delete_country_profile(e, &account, index);
    }

    fn get_country_profile(e: &Env, account: Address, index: u32) -> CountryProfile {
        irs::get_country_profile(e, &account, index)
    }

    fn get_country_profiles(e: &Env, account: Address) -> Vec<CountryProfile> {
        irs::get_country_profiles(e, &account)
    }

    fn get_country_profile_count(e: &Env, account: Address) -> u32 {
        irs::get_country_profile_count(e, &account)
    }
}
