use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use crate::rwa::{
    identity_registry_storage::{
        storage, CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
        IdentityType,
    },
    utils::token_binder::{bind_token, linked_tokens, unbind_token, TokenBinder},
};

pub struct IdentityRegistryStorageContract;

impl TokenBinder for IdentityRegistryStorageContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        linked_tokens(e)
    }

    fn bind_token(e: &Env, token: Address, operator: Address) {
        operator.require_auth();
        bind_token(e, &token);
    }

    fn unbind_token(e: &Env, token: Address, operator: Address) {
        operator.require_auth();
        unbind_token(e, &token);
    }
}

impl IdentityRegistryStorage for IdentityRegistryStorageContract {
    type CountryData = CountryData;

    // NOTE: IdentityType is set to Individual here.
    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        country_data_list: Vec<Self::CountryData>,
        operator: Address,
    ) {
        operator.require_auth();
        storage::add_identity(e, &account, &identity, IdentityType::Individual, &country_data_list);
    }

    fn remove_identity(e: &Env, account: Address, operator: Address) {
        operator.require_auth();
        storage::remove_identity(e, &account);
    }

    fn modify_identity(e: &Env, account: Address, identity: Address, operator: Address) {
        operator.require_auth();
        storage::modify_identity(e, &account, &identity);
    }

    fn recover_identity(e: &Env, old_account: Address, new_account: Address, operator: Address) {
        operator.require_auth();
        storage::recover_identity(e, &old_account, &new_account);
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        storage::stored_identity(e, &account)
    }

    fn get_recovered_to(e: &Env, old_account: Address) -> Option<Address> {
        storage::get_recovered_to(e, &old_account)
    }
}

impl CountryDataManager for IdentityRegistryStorageContract {
    fn add_country_data_entries(
        e: &Env,
        account: Address,
        country_data_list: Vec<Self::CountryData>,
        operator: Address,
    ) {
        operator.require_auth();
        storage::add_country_data_entries(e, &account, &country_data_list);
    }

    fn modify_country_data(
        e: &Env,
        account: Address,
        index: u32,
        country_data: Self::CountryData,
        operator: Address,
    ) {
        operator.require_auth();
        storage::modify_country_data(e, &account, index, &country_data);
    }

    fn delete_country_data(e: &Env, account: Address, index: u32, operator: Address) {
        operator.require_auth();
        storage::delete_country_data(e, &account, index);
    }

    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Self::CountryData> {
        storage::get_country_data_entries(e, &account)
    }

    fn get_country_data(e: &Env, account: Address, index: u32) -> Self::CountryData {
        storage::get_country_data(e, &account, index)
    }
}
