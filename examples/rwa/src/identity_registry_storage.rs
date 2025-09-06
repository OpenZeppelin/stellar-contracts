//! # Identity Registry Storage Contract
//!
//! Manages identity profiles and country data for RWA token compliance.
//! This contract stores the mapping between wallet addresses and their
//! associated identity contracts with jurisdictional information.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_admin, only_role};
use stellar_tokens::rwa::{
    identity_registry_storage::{
        add_country_data_entries, add_identity, delete_country_data, get_country_data,
        get_country_data_entries, get_identity, modify_country_data, modify_identity,
        remove_identity, CountryData, CountryDataManager, IdentityRegistryStorage, IdentityType,
    },
    utils::token_binder::{bind_token, linked_tokens, unbind_token, TokenBinder},
};

/// Role for identity managers
pub const IDENTITY_MANAGER_ROLE: soroban_sdk::Symbol = symbol_short!("ID_MGR");

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    /// Initializes the identity registry contract
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &admin, &IDENTITY_MANAGER_ROLE);
    }
}

#[contractimpl]
impl TokenBinder for IdentityRegistryContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        linked_tokens(e)
    }

    #[only_admin]
    fn bind_token(e: &Env, token: Address, _operator: Address) {
        bind_token(e, &token);
    }

    #[only_admin]
    fn unbind_token(e: &Env, token: Address, _operator: Address) {
        unbind_token(e, &token);
    }
}

#[contractimpl]
impl IdentityRegistryStorage for IdentityRegistryContract {
    type CountryData = CountryData;

    #[only_role(operator, "ID_MGR")]
    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        country_data_list: Vec<Self::CountryData>,
        operator: Address,
    ) {
        add_identity(e, &account, &identity, IdentityType::Individual, &country_data_list);
    }

    #[only_role(operator, "ID_MGR")]
    fn modify_identity(e: &Env, account: Address, identity: Address, operator: Address) {
        modify_identity(e, &account, &identity);
    }

    #[only_role(operator, "ID_MGR")]
    fn remove_identity(e: &Env, account: Address, operator: Address) {
        remove_identity(e, &account);
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        get_identity(e, &account)
    }
}

#[contractimpl]
impl CountryDataManager for IdentityRegistryContract {
    #[only_role(operator, "ID_MGR")]
    fn add_country_data_entries(
        e: &Env,
        account: Address,
        country_data_list: Vec<Self::CountryData>,
        operator: Address,
    ) {
        add_country_data_entries(e, &account, &country_data_list);
    }

    #[only_role(operator, "ID_MGR")]
    fn modify_country_data(
        e: &Env,
        account: Address,
        index: u32,
        country_data: Self::CountryData,
        operator: Address,
    ) {
        modify_country_data(e, &account, index, &country_data);
    }

    #[only_role(operator, "ID_MGR")]
    fn delete_country_data(e: &Env, account: Address, index: u32, operator: Address) {
        delete_country_data(e, &account, index);
    }

    fn get_country_data(e: &Env, account: Address, index: u32) -> Self::CountryData {
        get_country_data(e, &account, index)
    }

    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Self::CountryData> {
        get_country_data_entries(e, &account)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for IdentityRegistryContract {}
