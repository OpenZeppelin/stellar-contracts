use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, FromVal, Symbol, Val, Vec};
use stellar_access::access_control::{self as access_control};
use stellar_macros::only_role;
use stellar_tokens::rwa::{
    identity_verification::identity_registry_storage::{
        self as identity_storage, CountryData, CountryDataManager, IdentityRegistryStorage,
        IdentityType,
    },
    utils::token_binder::{self as binder, TokenBinder},
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }

    #[only_role(operator, "manager")]
    pub fn bind_tokens(e: &Env, tokens: Vec<Address>, operator: Address) {
        binder::bind_tokens(e, &tokens);
    }

    pub fn get_token_index(e: &Env, token: Address) -> u32 {
        binder::get_token_index(e, &token)
    }
}

#[contractimpl(contracttrait)]
impl TokenBinder for IdentityRegistryContract {
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
        identity_storage::add_identity(
            e,
            &account,
            &identity,
            IdentityType::Individual,
            &Vec::from_iter(e, initial_profiles.iter().map(|p| CountryData::from_val(e, &p))),
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
        identity_storage::add_country_data_entries(
            e,
            &account,
            &Vec::from_iter(e, profiles.iter().map(|p| CountryData::from_val(e, &p))),
        );
    }

    #[only_role(operator, "manager")]
    fn modify_country_data(e: &Env, account: Address, index: u32, profile: Val, operator: Address) {
        identity_storage::modify_country_data(
            e,
            &account,
            index,
            &CountryData::from_val(e, &profile),
        );
    }

    #[only_role(operator, "manager")]
    fn delete_country_data(e: &Env, account: Address, index: u32, operator: Address) {
        identity_storage::delete_country_data(e, &account, index);
    }
}
