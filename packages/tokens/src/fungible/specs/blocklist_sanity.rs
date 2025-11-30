use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{
    FungibleToken, blocklist::FungibleBlockList, specs::fungible_blocklist_contract::*
};


#[rule]
pub fn block_list_total_supply(e: Env) {
    let _ = FungibleBlockListContract::total_supply(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_balance(e: Env) {
    let account = nondet_address();
    let _ = FungibleBlockListContract::balance(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_allowance(e: Env) {
    let owner = nondet_address();
    let spender = nondet_address();
    let _ = FungibleBlockListContract::allowance(&e, owner, spender);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_transfer(e: Env) {
    let to = nondet_address();
    let from = nondet_address();
    let amount = nondet();
    FungibleBlockListContract::transfer(&e, from, to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_transfer_from(e: Env) {
    let spender = nondet_address();
    let to = nondet_address();
    let from = nondet_address();
    let amount = nondet();
    FungibleBlockListContract::transfer_from(&e, spender, from, to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_approve(e: Env) {
    let owner = nondet_address();
    let spender = nondet_address();
    let amount = nondet();
    let until = nondet();
    FungibleBlockListContract::approve(&e, owner, spender, amount, until);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_decimals(e: Env) {
    FungibleBlockListContract::decimals(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_name(e: Env) {
    FungibleBlockListContract::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_list_symbol(e: Env) {
    FungibleBlockListContract::symbol(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn blocked_sanity(e: Env) {
    let account: Address = nondet_address();
    let _ = FungibleBlockListContract::blocked(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn block_user_sanity(e: Env) {
    let user: Address = nondet_address();
    let operator: Address = nondet_address();
    FungibleBlockListContract::block_user(&e, user, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn unblock_user_sanity(e: Env) {
    let user: Address = nondet_address();
    let operator: Address = nondet_address();
    FungibleBlockListContract::unblock_user(&e, user, operator);
    cvlr_satisfy!(true);
}