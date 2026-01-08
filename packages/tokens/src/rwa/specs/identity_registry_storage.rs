use soroban_sdk::{Address, Env};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use crate::rwa::specs::helpers::nondet::nondet_vec_country;
use cvlr_soroban_derive::rule;
use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use crate::rwa::identity_registry_storage::storage::{
    get_identity_profile, get_country_data_entries, get_country_data, stored_identity, get_recovered_to,
    add_identity, remove_identity, modify_identity, recover_identity, modify_country_data, add_country_data_entries, delete_country_data
};
use crate::rwa::specs::helpers::nondet;
use crate::rwa::identity_registry_storage::storage::IdentityType;
use crate::rwa::identity_registry_storage::storage::IRSStorageKey;
use crate::rwa::identity_registry_storage::storage::IdentityProfile;

// helpers

pub fn get_stored_identity_non_pancicking(e: Env, account: Address) -> Option<Address> {
    let key = IRSStorageKey::Identity(account.clone());
    e.storage().persistent().get(&key)
}

pub fn get_identity_profile_non_pancicking(e: Env, account: Address) -> Option<IdentityProfile> {
    let key = IRSStorageKey::IdentityProfile(account.clone());
    e.storage().persistent().get(&key)
}

pub fn get_recovered_to_non_pancicking(e: Env, account: Address) -> Option<Address> {
    let key = IRSStorageKey::RecoveredTo(account.clone());
    e.storage().persistent().get(&key)
}

// functions in the trait

#[rule]
// after add_identity the stored identity is some
// status: violation
pub fn add_identity_integrity_1(e: Env) {
    let account: Address = nondet_address();
    let identity = nondet_address();
    let identity_type: IdentityType = nondet();
    let initial_countries = nondet_vec_country();
    let key = IRSStorageKey::Identity(account.clone());
    let stored_identity: Option<Address> = get_stored_identity_non_pancicking(e, account);
    cvlr_assert!(stored_identity.is_some());
}

#[rule]
// after add_identity the stored_identity is the given identity
// status: verified
pub fn add_identity_integrity_2(e: Env) {
    let account: Address = nondet_address();
    let identity = nondet_address();
    let identity_type = nondet();
    let initial_countries = nondet_vec_country();
    add_identity(&e, &account, &identity, identity_type, &initial_countries);
    let stored_identity = stored_identity(&e, &account);
    cvlr_assert!(stored_identity == identity);
}

#[rule]
// after add_identity the identity_profile has the same identity_type
// status: ?
pub fn add_identity_integrity_3(e: Env) {
    let account: Address = nondet_address();
    let identity = nondet_address();
    let identity_type: IdentityType = nondet();
    let initial_countries = nondet_vec_country();
    add_identity(&e, &account, &identity, identity_type.clone(), &initial_countries);
    let identity_profile = get_identity_profile(&e, &account);
    let identity_profile_type = identity_profile.identity_type;
    cvlr_assert!(identity_profile_type == identity_type);
}

#[rule]
// after add_identity the identity_profile has the same number of countries as the initial_countries
// status: ?
pub fn add_identity_integrity_4(e: Env) {
    let account: Address = nondet_address();
    let identity = nondet_address();
    let identity_type: IdentityType = nondet();
    let initial_countries = nondet_vec_country();
    add_identity(&e, &account, &identity, identity_type.clone(), &initial_countries);
    let identity_profile = get_identity_profile(&e, &account);
    let identity_profile_countries = identity_profile.countries;
    cvlr_assert!(identity_profile_countries == initial_countries);
}

#[rule]
// after remove_identity the stored identity is none
// status: verified
pub fn remove_identity_integrity_1(e: Env) {
    let account: Address = nondet_address();
    remove_identity(&e, &account);
    let stored_identity: Option<Address> = get_stored_identity_non_pancicking(e, account);
    cvlr_assert!(stored_identity.is_none());
}

#[rule]
// after remove_identity the identity_profile is none
// status: verified
pub fn remove_identity_integrity_2(e: Env) {
    let account: Address = nondet_address();
    remove_identity(&e, &account);
    let identity_profile: Option<IdentityProfile> = get_identity_profile_non_pancicking(e, account);
    cvlr_assert!(identity_profile.is_none());
}

#[rule]
// after modify_identity the identity changes
// status: verified
pub fn modify_identity_integrity_1(e: Env) {
    let account: Address = nondet_address();
    let new_identity = nondet_address();
    modify_identity(&e, &account, &new_identity);
    let stored_identity = stored_identity(&e, &account);
    cvlr_assert!(stored_identity == new_identity);
}

#[rule]
// after modify_identity the identity_profile is the same
// status: spurious violation
pub fn modify_identity_integrity_2(e: Env) {
    let account: Address = nondet_address();
    let identity_profile_pre = get_identity_profile(&e, &account);
    let new_identity = nondet_address();
    modify_identity(&e, &account, &new_identity);
    let identity_profile_post = get_identity_profile(&e, &account);
    cvlr_assert!(identity_profile_post == identity_profile_pre);
}

#[rule]
// after recover_identity the identity moves from old_account to new_account
// status: verified
pub fn recover_identity_integrity_1(e: Env) {
    let old_account: Address = nondet_address();
    let new_account: Address = nondet_address();
    let store_identity_pre_old_account = stored_identity(&e, &old_account);
    recover_identity(&e, &old_account, &new_account);
    let stored_identity_post_new_account = stored_identity(&e, &new_account);
    cvlr_assert!(stored_identity_post_new_account == store_identity_pre_old_account);
}

#[rule]
// after recover_identity the identity_profile moves from old_account to new_account
// status: spurious violation
pub fn recover_identity_integrity_2(e: Env) {
    let old_account: Address = nondet_address();
    let new_account: Address = nondet_address();
    let identity_profile_pre_old_account = get_identity_profile(&e, &old_account);
    recover_identity(&e, &old_account, &new_account);
    let identity_profile_post_new_account = get_identity_profile(&e, &new_account);
    cvlr_assert!(identity_profile_post_new_account == identity_profile_pre_old_account);
}

#[rule]
// after recover_identity the recovered_to is set to new_account
// status: verified
pub fn recover_identity_integrity_3(e: Env) {
    let old_account: Address = nondet_address();
    let new_account: Address = nondet_address();
    recover_identity(&e, &old_account, &new_account);
    let recovered_to = get_recovered_to(&e, &old_account);
    cvlr_assert!(recovered_to == Some(new_account));
}

#[rule]
// after recover_identity the stored_identity of the old account is none
// status: verified
pub fn recover_identity_integrity_4(e: Env) {
    let old_account: Address = nondet_address();
    let new_account: Address = nondet_address();
    recover_identity(&e, &old_account, &new_account);
    let stored_identity = get_stored_identity_non_pancicking(e, old_account);
    cvlr_assert!(stored_identity.is_none());
}

#[rule]
// after recover_identity the identity_profile of the old account is none
// status: verified
pub fn recover_identity_integrity_5(e: Env) {
    let old_account: Address = nondet_address();
    let new_account: Address = nondet_address();
    recover_identity(&e, &old_account, &new_account);
    let identity_profile = get_identity_profile_non_pancicking(e, old_account);
    cvlr_assert!(identity_profile.is_none());
}

#[rule]
// after add_country_data_entries the length is added
// status: ?
pub fn add_country_data_entries_integrity_1(e: Env) {
    let account: Address = nondet_address();
    let country_data_entries = nondet_vec_country();
    let added_length = country_data_entries.len();
    let length_pre = get_country_data_entries(&e, &account).len();
    add_country_data_entries(&e, &account, &country_data_entries);
    let length_post = get_country_data_entries(&e, &account).len();
    cvlr_assert!(length_post == length_pre + added_length);
}

#[rule]
// after add_country_data_entries any data added entry is in the country data entries
// status: ?
pub fn add_country_data_entries_integrity_2(e: Env) {
    let account: Address = nondet_address();
    let country_data_entries = nondet_vec_country();
    let added_country = nondet();
    cvlr_assume!(country_data_entries.contains(&added_country));
    add_country_data_entries(&e, &account, &country_data_entries);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let added_country_in_entries_post = country_data_entries_post.contains(&added_country);
    cvlr_assert!(added_country_in_entries_post);
}

#[rule]
// after modify_country_data the country_data in given index is some
// status: verified
pub fn modify_country_data_integrity_1(e: Env) {
    let account: Address = nondet_address();
    let index: u32 = nondet();
    let country_data = nondet();
    modify_country_data(&e, &account, index, &country_data);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let country_data_in_entries_post = country_data_entries_post.get(index);
    cvlr_assert!(country_data_in_entries_post.is_some());
}

#[rule]
// after modify_country_data the country_data in given index is the input
// status: violation
pub fn modify_country_data_integrity_2(e: Env) {
    let account: Address = nondet_address();
    let index: u32 = nondet();
    let country_data = nondet();
    modify_country_data(&e, &account, index, &country_data);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let country_data_in_entries_post = country_data_entries_post.get(index);
    cvlr_assert!(country_data_in_entries_post == Some(country_data));
}

#[rule]
// after modify_country_data the length is unchanged
// status: verified
pub fn modify_country_data_integrity_3(e: Env) {
    let account: Address = nondet_address();
    let index: u32 = nondet();
    let country_data = nondet();
    let length_pre = get_country_data_entries(&e, &account).len();
    modify_country_data(&e, &account, index, &country_data);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let length_post = country_data_entries_post.len();
    cvlr_assert!(length_post == length_pre);
}

#[rule]
// after modify_country_data the data is any other index is unchanged
// status: violation
pub fn modify_country_data_integrity_4(e: Env) {
    let account: Address = nondet_address();
    let index: u32 = nondet();
    let country_data = nondet();
    let index_other = nondet();
    cvlr_assume!(index_other != index);
    let country_data_entries_pre = get_country_data_entries(&e, &account);
    let entry_pre_index_other = country_data_entries_pre.get(index_other);
    modify_country_data(&e, &account, index, &country_data);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let entry_post_index_other = country_data_entries_post.get(index_other);
    cvlr_assert!(entry_post_index_other == entry_pre_index_other);
}

#[rule]
// after delete_country_data the country_data in index is none
// status: violation
pub fn delete_country_data_integrity_1(e: Env) {
// after delete_country_data the country_data in index is none
    let account: Address = nondet_address();
    let index: u32 = nondet();
    delete_country_data(&e, &account, index);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let country_data_in_entries_post = country_data_entries_post.get(index);
    cvlr_assert!(country_data_in_entries_post.is_none());
}

#[rule]
// after_delete_country_data the length is decreased by 1
// status: verified
pub fn delete_country_data_integrity_2(e: Env) {
    let account: Address = nondet_address();
    let index: u32 = nondet();
    let length_pre = get_country_data_entries(&e, &account).len();
    delete_country_data(&e, &account, index);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let length_post = country_data_entries_post.len();
    cvlr_assert!(length_post == length_pre - 1);
}

#[rule]
// after delete_country_data the data in any other index is unchanged
// status: violation
pub fn delete_country_data_integrity_3(e: Env) {
    let account: Address = nondet_address();
    let index: u32 = nondet();
    let index_other = nondet();
    cvlr_assume!(index_other != index);
    let country_data_entries_pre = get_country_data_entries(&e, &account);
    let entry_pre_index_other = country_data_entries_pre.get(index_other);
    delete_country_data(&e, &account, index);
    let country_data_entries_post = get_country_data_entries(&e, &account);
    let entry_post_index_other = country_data_entries_post.get(index_other);
    cvlr_assert!(entry_post_index_other == entry_pre_index_other);
}

// todo
// invariants 
// there are a few checks that should hold for all identities as an invariant
// identity exists iff identity profile exists
