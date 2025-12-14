use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    enumerable::Enumerable,
    extensions::enumerable::storage::{NFTEnumerableStorageKey, OwnerTokensKey},
    overrides::ContractOverrides,
    Base, OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT, TOKEN_TTL_THRESHOLD,
};
use crate::non_fungible::utils::sequential;
use crate::non_fungible::specs::helper::is_owned;
// helpers

/// Returns the `token_id` owned by `owner` at a given `index` in the
/// owner's local list, or `None` if not found. This is a non-panicking
/// version of [`Enumerable::get_owner_token_id`].
pub fn try_get_owner_token_id(e: &Env, owner: &Address, index: u32) -> Option<u32> {
    let key = NFTEnumerableStorageKey::OwnerTokens(OwnerTokensKey { owner: owner.clone(), index });
    let Some(token_id) = e.storage().persistent().get::<_, u32>(&key) else {
        return None;
    };
    e.storage().persistent().extend_ttl(&key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);
    Some(token_id)
}

pub fn try_get_owner_token_index(e: &Env, owner: &Address, token_id: u32) -> Option<u32> {
    let key = NFTEnumerableStorageKey::OwnerTokensIndex(token_id);
    let Some(index) = e.storage().persistent().get::<_, u32>(&key) else {
        return None;
    };
    e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);
    Some(index)
}

/// Returns the `token_id` at a given `index` in the global token list,
/// or `None` if not found. This is a non-panicking version of
/// [`Enumerable::get_token_id`].
pub fn try_get_token_id(e: &Env, index: u32) -> Option<u32> {
    let key = NFTEnumerableStorageKey::GlobalTokens(index);
    let Some(token_id) = e.storage().persistent().get::<_, u32>(&key) else {
        return None;
    };
    e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);

    Some(token_id)
}

pub fn try_get_token_index(e: &Env, token_id: u32) -> Option<u32> {
    let key = NFTEnumerableStorageKey::GlobalTokensIndex(token_id);
    let Some(index) = e.storage().persistent().get::<_, u32>(&key) else {
        return None;
    };
    e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);
    Some(index)
}

// ################## INVARIANTS ##################

// this is very similar to the pattern we have in access_control, but we have spurious violations there.
// get back to this after finishing access_control invariants.


// invariants should be checked for transfer, transfer_from, mint,
// sequential_mint, burn and burn_from (approves are trivial)
// todo
// invariant: index < balance <-> get_owner_token_id != none
// invariant: consistent mapping invariant
// maybe we also need an injectivity invariant like in access_control.

// invariant: index < total_supply <-> get_token_id != none

// helpers

pub fn assume_pre_valid_index(e: Env, index: u32) {
    clog!(index);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let index_less_than_total_supply = index < total_supply;
    clog!(index_less_than_total_supply);
    let token_id = try_get_token_id(&e, index);
    clog!(token_id);
    let token_id_not_none = token_id.is_some();
    clog!(token_id_not_none);
    cvlr_assume!(index_less_than_total_supply == token_id_not_none);
}

pub fn assert_post_valid_index(e: Env, index: u32) {
    clog!(index);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let index_less_than_total_supply = index < total_supply;
    clog!(index_less_than_total_supply);
    let token_id = try_get_token_id(&e, index);
    clog!(token_id);
    let token_id_not_none = token_id.is_some();
    clog!(token_id_not_none);
    cvlr_assert!(index_less_than_total_supply == token_id_not_none);
}

#[rule]
// status: verified
// note: 4 minutes
pub fn after_nft_transfer_valid_index(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::transfer(&e, &from, &to, token_id);
    assert_post_valid_index(e, index);
}

#[rule]
// status: verified
// note: 12 minutes
pub fn after_nft_transfer_from_valid_index(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::transfer_from(&e, &spender, &from, &to, token_id);
    assert_post_valid_index(e, index);
}

#[rule]
// status: verified
// note: 54 minutes
pub fn after_nft_non_sequential_mint_valid_index(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::non_sequential_mint(&e, &to, token_id);
    assert_post_valid_index(e, index);
}

#[rule]
// status: verified
// note: 53 minutes
pub fn after_nft_sequential_mint_valid_index(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let index = u32::nondet();
    clog!(index);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::sequential_mint(&e, &to);
    assert_post_valid_index(e, index);
}

#[rule]
// status: violation - missing invariant wip below
pub fn after_nft_burn_valid_index(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::burn(&e, &from, token_id);
    assert_post_valid_index(e, index);
}

#[rule]
// status: violation - missing invariant wip below
pub fn after_nft_burn_from_valid_index(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::burn_from(&e, &spender, &from, token_id);
    assert_post_valid_index(e, index);
}

// helpers

pub fn assume_pre_consistent_mappings(e: Env, index: u32, token_id: u32) {
    clog!(index);
    clog!(token_id);
    let token_id_from_index = try_get_token_id(&e, index);
    clog!(token_id_from_index);
    let index_from_token_id = try_get_token_index(&e, token_id);
    clog!(index_from_token_id);
    let token_id_from_index_equals_token_id = token_id_from_index == Some(token_id);
    clog!(token_id_from_index_equals_token_id);
    let index_from_token_id_equals_index = index_from_token_id == Some(index);
    clog!(index_from_token_id_equals_index);
    cvlr_assume!(token_id_from_index_equals_token_id == index_from_token_id_equals_index);
}

pub fn assert_post_consistent_mappings(e: Env, index: u32, token_id: u32) {
    clog!(index);
    clog!(token_id);
    let token_id_from_index = try_get_token_id(&e, index);
    clog!(token_id_from_index);
    let index_from_token_id = try_get_token_index(&e, token_id);
    clog!(index_from_token_id);
    let token_id_from_index_equals_token_id = token_id_from_index == Some(token_id);
    clog!(token_id_from_index_equals_token_id);
    let index_from_token_id_equals_index = index_from_token_id == Some(index);
    clog!(index_from_token_id_equals_index);
    cvlr_assert!(token_id_from_index_equals_token_id == index_from_token_id_equals_index);
}

#[rule]
// status: verified
pub fn after_nft_transfer_consistent_mappings(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let transferred_token = nondet();
    clog!(transferred_token);
    let index = u32::nondet();
    clog!(index);
    assume_pre_consistent_mappings(e.clone(), index, token_id);
    Enumerable::transfer(&e, &from, &to, transferred_token);
    assert_post_consistent_mappings(e, index, token_id);
}

#[rule]
// status: verified
pub fn after_nft_transfer_from_consistent_mappings(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let transferred_token = nondet();
    clog!(transferred_token);
    let index = u32::nondet();
    clog!(index);
    assume_pre_consistent_mappings(e.clone(), index, token_id);
    Enumerable::transfer_from(&e, &spender, &from, &to, transferred_token);
    assert_post_consistent_mappings(e, index, token_id);
}

#[rule]
// status:
pub fn after_nft_non_sequential_mint_consistent_mappings(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = nondet();
    clog!(token_id);
    let minted_token_id= nondet();
    clog!(minted_token_id);
    let minted_token_is_owned = is_owned(&e, minted_token_id);
    clog!(minted_token_is_owned);
    cvlr_assume!(!minted_token_is_owned); // assumes token_id has not been minted before.
    let index = nondet();
    clog!(index);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    assume_pre_valid_index(e.clone(), index);
    assume_pre_consistent_mappings(e.clone(), index, token_id);
    Enumerable::non_sequential_mint(&e, &to, minted_token_id);
    assert_post_consistent_mappings(e, index, token_id);
}

#[rule]
// status:
pub fn after_nft_sequential_mint_consistent_mappings(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let next_token_id = sequential::next_token_id(&e);
    clog!(next_token_id);
    assume_pre_next_token_id_geq_total_supply(e.clone()); // assumes only sequential mints.
    assume_pre_valid_index(e.clone(), index);
    assume_pre_consistent_mappings(e.clone(), index, token_id);
    Enumerable::sequential_mint(&e, &to);
    assert_post_consistent_mappings(e, index, token_id);
}

#[rule]
// status: need more invariants / instantations cex is unreachable.
pub fn after_nft_burn_consistent_mappings(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    let burned_token_id = nondet();
    clog!(burned_token_id);
    assume_pre_consistent_mappings(e.clone(), index, token_id);
    assume_pre_consistent_mappings(e.clone(), index, burned_token_id);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::burn(&e, &from, burned_token_id); 
    assert_post_consistent_mappings(e, index, token_id);
}

#[rule]
// status:
pub fn after_nft_burn_from_consistent_mappings(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let index = u32::nondet();
    clog!(index);
    let burned_token_id = nondet();
    clog!(burned_token_id);
    assume_pre_consistent_mappings(e.clone(), index, token_id);
    assume_pre_consistent_mappings(e.clone(), index, burned_token_id);
    assume_pre_valid_index(e.clone(), index);
    Enumerable::burn_from(&e, &spender, &from, burned_token_id);
    assert_post_consistent_mappings(e, index, token_id);
}

// invariant: next_token_id >= total_supply()
// interesting only for sequential mint
// not equality because maybe there are burns

// helper

pub fn assume_pre_next_token_id_geq_total_supply(e: Env) {
    let next_token_id = sequential::next_token_id(&e);
    clog!(next_token_id);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    cvlr_assume!(next_token_id >= total_supply);
}

pub fn assert_post_next_token_id_geq_total_supply(e: Env) {
    let next_token_id = sequential::next_token_id(&e);
    clog!(next_token_id);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    cvlr_assert!(next_token_id >= total_supply);
}

#[rule]
// status: 
// this assumes that we have only sequential mints
pub fn after_nft_sequential_mint_next_token_id_geq_total_supply(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    assume_pre_next_token_id_geq_total_supply(e.clone());
    Enumerable::sequential_mint(&e, &to);
    assert_post_next_token_id_geq_total_supply(e);
}

#[rule]
// status:
pub fn after_nft_burn_next_token_id_geq_total_supply(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let burned_token_id = nondet();
    clog!(burned_token_id);
    assume_pre_next_token_id_geq_total_supply(e.clone());
    Enumerable::burn(&e, &from, burned_token_id);
    assert_post_next_token_id_geq_total_supply(e);
}

#[rule]
// status:
pub fn after_nft_burn_from_next_token_id_geq_total_supply(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let burned_token_id = nondet();
    clog!(burned_token_id);
    assume_pre_next_token_id_geq_total_supply(e.clone());
    Enumerable::burn_from(&e, &spender, &from, burned_token_id);
    assert_post_next_token_id_geq_total_supply(e);
}

// invariant: token_id_index.is_some() <-> is_owned

// helpers

pub fn assume_pre_token_id_iff_owned(e: Env, token_id: u32) {
    let token_id_is_owned = is_owned(&e, token_id);
    clog!(token_id_is_owned);
    let token_id_index = try_get_token_index(&e, token_id);
    clog!(token_id_index);
    let token_id_index_is_some = token_id_index.is_some();
    clog!(token_id_index_is_some);
    cvlr_assume!(token_id_is_owned == token_id_index_is_some);
}

pub fn assert_post_token_id_iff_owned(e: Env, token_id: u32) {
    let token_id_is_owned = is_owned(&e, token_id);
    clog!(token_id_is_owned);
    let token_id_index = try_get_token_index(&e, token_id);
    clog!(token_id_index);
    let token_id_index_is_some = token_id_index.is_some();
    clog!(token_id_index_is_some);
    cvlr_assert!(token_id_is_owned == token_id_index_is_some);
}

#[rule]
// status: verified
pub fn after_nft_transfer_token_id_iff_owned(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let transferred_token = nondet();
    clog!(transferred_token);
    assume_pre_token_id_iff_owned(e.clone(), token_id);
    Enumerable::transfer(&e, &from, &to, transferred_token);
    assert_post_token_id_iff_owned(e, token_id);
}

#[rule]
// status: verified
pub fn after_nft_transfer_from_token_id_iff_owned(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let transferred_token = nondet();
    clog!(transferred_token);
    assume_pre_token_id_iff_owned(e.clone(), token_id);
    Enumerable::transfer_from(&e, &spender, &from, &to, transferred_token);
    assert_post_token_id_iff_owned(e, token_id);
}

#[rule]
// status: verified
pub fn after_nft_non_sequential_mint_token_id_iff_owned(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let minted_token = nondet();
    clog!(minted_token);
    assume_pre_token_id_iff_owned(e.clone(), token_id);
    Enumerable::non_sequential_mint(&e, &to, minted_token);
    assert_post_token_id_iff_owned(e, token_id);
}

#[rule]
// status: verified
pub fn after_nft_sequential_mint_token_id_iff_owned(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    assume_pre_token_id_iff_owned(e.clone(), token_id);
    Enumerable::sequential_mint(&e, &to);
    assert_post_token_id_iff_owned(e, token_id);
}

#[rule]
// status:
pub fn after_nft_burn_token_id_iff_owned(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let burned_token = nondet();
    clog!(burned_token);
    assume_pre_token_id_iff_owned(e.clone(), token_id);
    Enumerable::burn(&e, &from, burned_token);
    assert_post_token_id_iff_owned(e, token_id);
}

#[rule]
// status:
pub fn after_nft_burn_from_token_id_iff_owned(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let burned_token = nondet();
    clog!(burned_token);
    assume_pre_token_id_iff_owned(e.clone(), token_id);
    Enumerable::burn_from(&e, &spender, &from, burned_token);
    assert_post_token_id_iff_owned(e, token_id);
}