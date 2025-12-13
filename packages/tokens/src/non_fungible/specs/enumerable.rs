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

// helpers

pub fn assume_pre_total_supply_geq_balance(e: Env, account: &Address) {
    clog!(cvlr_soroban::Addr(account));
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let balance = Enumerable::balance(&e, account);
    clog!(balance);
    cvlr_assume!(total_supply >= balance);
}

pub fn assert_post_total_supply_geq_balance(e: Env, account: &Address) {
    clog!(cvlr_soroban::Addr(account));
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let balance = Enumerable::balance(&e, account);
    clog!(balance);
    cvlr_assert!(total_supply >= balance);
}

#[rule]
// status: violation - bad rule
// https://prover.certora.com/output/5771024/c6c53c3cbf134b41a6dccc5a68a30cff/
pub fn after_nft_transfer_total_supply_geq_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    assume_pre_total_supply_geq_balance(e.clone(), &from);
    Enumerable::transfer(&e, &from, &to, token_id);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: violation bad rule
pub fn after_nft_transfer_from_total_supply_geq_balance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    assume_pre_total_supply_geq_balance(e.clone(), &from);
    Enumerable::transfer_from(&e, &spender, &from, &to, token_id);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: verified
pub fn after_nft_non_sequential_mint_total_supply_geq_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let token_id = u32::nondet();
    clog!(token_id);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    Enumerable::non_sequential_mint(&e, &to, token_id);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: verified
pub fn after_nft_sequential_mint_total_supply_geq_balance(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    Enumerable::sequential_mint(&e, &to);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: violation - same problem from fungible. bad rule
// https://prover.certora.com/output/33158/b15351401b0e464795731ba47f482125
pub fn after_nft_burn_total_supply_geq_balance(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    Enumerable::burn(&e, &from, token_id);
    assert_post_total_supply_geq_balance(e, &account);
}

#[rule]
// status: violation https://prover.certora.com/output/33158/6ec050fb37a14a48980ecd71a718df04
// - expected violation, as in burn bad rule
pub fn after_nft_burn_from_total_supply_geq_balance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let token_id = u32::nondet();
    clog!(token_id);
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    assume_pre_total_supply_geq_balance(e.clone(), &account);
    assume_pre_total_supply_geq_balance(e.clone(), &from);
    Enumerable::burn_from(&e, &spender, &from, token_id);
    assert_post_total_supply_geq_balance(e, &account);
}

// invariant: index < total_supply <-> get_token_id != none

// helpers

pub fn assume_pre_valid_index(e: Env, index: u32) {
    clog!(index);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let index_less_than_total_supply = index < total_supply;
    let token_id = try_get_token_id(&e, index);
    let token_id_not_none = token_id.is_some();
    cvlr_assume!(index_less_than_total_supply == token_id_not_none);
}

pub fn assert_post_valid_index(e: Env, index: u32) {
    clog!(index);
    let total_supply = Enumerable::total_supply(&e);
    clog!(total_supply);
    let index_less_than_total_supply = index < total_supply;
    let token_id = try_get_token_id(&e, index);
    let token_id_not_none = token_id.is_some();
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
// status: violation - investigate
// https://prover.certora.com/output/33158/75d3df1bfcce4f8d8c89280e4a10d046
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
// status: violation
// investigate https://prover.certora.com/output/33158/842685020d6b433197bb4efdb12558c6
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
