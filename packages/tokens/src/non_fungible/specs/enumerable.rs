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

// ################## INVARIANTS ##################

// invariant: index < balance <-> get_owner_token_id != none

// invariants should be checked for transfer, transfer_from, mint,
// sequential_mint, burn and burn_from (approves are trivial)

// invariant: total_supply >= balance()
// can't prove without ghosts and hooks

// invariant: index < total_supply <-> get_token_id != none

// TODO: supporting invariant about consistency of two mappings.
// index -> token
// token -> index
// then you need the same for balances

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
// status: violation - missing invariant
// https://prover.certora.com/output/5771024/752d106807eb450385e8fb72bb6d4d82/
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
// status: violation - missing invariant
// https://prover.certora.com/output/33158/842685020d6b433197bb4efdb12558c6
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
