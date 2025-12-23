use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Vec};

use crate::rwa::{
    identity_registry_storage::{
        CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
        IndividualCountryRelation,
    },
    specs::{identity_registry_storage::IdentityRegistryStorageContract, nondet::nondet_vec_country},
};

#[rule]
pub fn add_identity_sanity(e: Env) {
    let account = nondet_address();
    let identity = nondet_address();
    let operator = account.clone();
    let initial = nondet_vec_country();
    IdentityRegistryStorageContract::add_identity(&e, account, identity, initial, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_identity_sanity(e: Env) {
    let account = nondet_address();
    let operator = account.clone();
    IdentityRegistryStorageContract::remove_identity(&e, account, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn modify_identity_sanity(e: Env) {
    let account = nondet_address();
    let new_identity = nondet_address();
    let operator = account.clone();
    IdentityRegistryStorageContract::modify_identity(&e, account, new_identity, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn recover_identity_sanity(e: Env) {
    let old_account = nondet_address();
    let new_account = nondet_address();
    let operator = old_account.clone();
    IdentityRegistryStorageContract::recover_identity(&e, old_account, new_account, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn stored_identity_sanity(e: Env) {
    let account = nondet_address();
    IdentityRegistryStorageContract::stored_identity(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_recovered_to_sanity(e: Env) {
    let account = nondet_address();
    IdentityRegistryStorageContract::get_recovered_to(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn add_country_data_entries_sanity(e: Env) {
    let account = nondet_address();
    let operator = account.clone();
    let data = nondet_vec_country();
    IdentityRegistryStorageContract::add_country_data_entries(&e, account, data, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn modify_country_data_sanity(e: Env) {
    let account = nondet_address();
    let index: u32 = nondet();
    let country_data = CountryData::nondet();
    let operator = account.clone();
    IdentityRegistryStorageContract::modify_country_data(
        &e,
        account,
        index,
        country_data,
        operator,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn delete_country_data_sanity(e: Env) {
    let account = nondet_address();
    let operator = account.clone();
    IdentityRegistryStorageContract::delete_country_data(&e, account, 0, operator);
    cvlr_satisfy!(true);
}
