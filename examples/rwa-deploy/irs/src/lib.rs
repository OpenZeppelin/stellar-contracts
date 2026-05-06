#![no_std]

use soroban_sdk::{
    contract, contractimpl, symbol_short, Address, Env, FromVal, IntoVal, Symbol, Val, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::{
    identity_registry_storage::{
        self as identity_storage, CountryData, CountryDataManager, IdentityRegistryStorage,
        IdentityType,
    },
    utils::token_binder::{self as binder, TokenBinder},
};

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &symbol_short!("manager"), &admin);
    }

    #[only_role(operator, "manager")]
    pub fn bind_tokens(e: &Env, tokens: Vec<Address>, operator: Address) {
        binder::bind_tokens(e, &tokens);
    }
}

#[contractimpl]
impl TokenBinder for IdentityRegistryContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        binder::linked_tokens(e)
    }

    #[only_role(operator, "manager")]
    fn bind_token(e: &Env, token: Address, operator: Address) {
        binder::bind_token(e, &token);
    }

    #[only_role(operator, "manager")]
    fn unbind_token(e: &Env, token: Address, operator: Address) {
        binder::unbind_token(e, &token);
    }
}

#[contractimpl]
impl IdentityRegistryStorage for IdentityRegistryContract {
    #[only_role(operator, "manager")]
    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        initial_profiles: Vec<Val>,
        operator: Address,
    ) {
        let country_data = Vec::from_iter(
            e,
            initial_profiles.iter().map(|profile| CountryData::from_val(e, &profile)),
        );
        identity_storage::add_identity(
            e,
            &account,
            &identity,
            IdentityType::Individual,
            &country_data,
        );
    }

    #[only_role(operator, "manager")]
    fn modify_identity(e: &Env, account: Address, new_identity: Address, operator: Address) {
        identity_storage::modify_identity(e, &account, &new_identity);
    }

    #[only_role(operator, "manager")]
    fn remove_identity(e: &Env, account: Address, operator: Address) {
        identity_storage::remove_identity(e, &account);
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        identity_storage::stored_identity(e, &account)
    }

    #[only_role(operator, "manager")]
    fn recover_identity(e: &Env, old_account: Address, new_account: Address, operator: Address) {
        identity_storage::recover_identity(e, &old_account, &new_account);
    }

    fn get_recovered_to(e: &Env, old: Address) -> Option<Address> {
        identity_storage::get_recovered_to(e, &old)
    }
}

#[contractimpl]
impl CountryDataManager for IdentityRegistryContract {
    #[only_role(operator, "manager")]
    fn add_country_data_entries(e: &Env, account: Address, profiles: Vec<Val>, operator: Address) {
        let country_data =
            Vec::from_iter(e, profiles.iter().map(|profile| CountryData::from_val(e, &profile)));
        identity_storage::add_country_data_entries(e, &account, &country_data);
    }

    #[only_role(operator, "manager")]
    fn modify_country_data(e: &Env, account: Address, index: u32, profile: Val, operator: Address) {
        let country_data = CountryData::from_val(e, &profile);
        identity_storage::modify_country_data(e, &account, index, &country_data);
    }

    #[only_role(operator, "manager")]
    fn delete_country_data(e: &Env, account: Address, index: u32, operator: Address) {
        identity_storage::delete_country_data(e, &account, index);
    }

    fn get_country_data(e: &Env, account: Address, index: u32) -> Val {
        identity_storage::get_country_data(e, &account, index).into_val(e)
    }

    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Val> {
        Vec::from_iter(
            e,
            identity_storage::get_country_data_entries(e, &account)
                .iter()
                .map(|profile| profile.into_val(e)),
        )
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for IdentityRegistryContract {}
