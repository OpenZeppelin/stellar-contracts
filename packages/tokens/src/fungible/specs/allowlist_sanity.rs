use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{
    allowlist::{AllowList, FungibleAllowList},
    burnable::FungibleBurnable,
    specs::fungible_allowlist_contract::*,
    FungibleToken,
};

#[rule]
pub fn allow_list_constructor(e: Env) {
    let cap = nondet();
    FungibleAllowListContract::__constructor(&e, cap);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_mint(e: Env) {
    let account = nondet_address();
    let amount = nondet();
    FungibleAllowListContract::mint(&e, account, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_total_supply(e: Env) {
    let _ = FungibleAllowListContract::total_supply(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_balance(e: Env) {
    let account = nondet_address();
    let _ = FungibleAllowListContract::balance(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_allowance(e: Env) {
    let owner = nondet_address();
    let spender = nondet_address();
    let _ = FungibleAllowListContract::allowance(&e, owner, spender);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_transfer(e: Env) {
    let to = nondet_address();
    let from = nondet_address();
    let amount = nondet();
    FungibleAllowListContract::transfer(&e, from, to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_transfer_from(e: Env) {
    let spender = nondet_address();
    let to = nondet_address();
    let from = nondet_address();
    let amount = nondet();
    FungibleAllowListContract::transfer_from(&e, spender, from, to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_approve(e: Env) {
    let owner = nondet_address();
    let spender = nondet_address();
    let amount = nondet();
    let until = nondet();
    FungibleAllowListContract::approve(&e, owner, spender, amount, until);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_decimals(e: Env) {
    FungibleAllowListContract::decimals(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_name(e: Env) {
    FungibleAllowListContract::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_symbol(e: Env) {
    FungibleAllowListContract::symbol(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allowed_sanity(e: Env) {
    let account: Address = nondet_address();
    let _ = FungibleAllowListContract::allowed(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_user_sanity(e: Env) {
    let user: Address = nondet_address();
    let operator: Address = nondet_address();
    FungibleAllowListContract::allow_user(&e, user, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn disallow_user_sanity(e: Env) {
    let user: Address = nondet_address();
    let operator: Address = nondet_address();
    FungibleAllowListContract::disallow_user(&e, user, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_burn_sanity(e: Env) {
    let from = nondet_address();
    let amount = nondet();
    FungibleAllowListContract::burn(&e, from, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn allow_list_burn_from_sanity(e: Env) {
    let from = nondet_address();
    let spender = nondet_address();
    let amount = nondet();
    FungibleAllowListContract::burn_from(&e, spender, from, amount);
    cvlr_satisfy!(true);
}
