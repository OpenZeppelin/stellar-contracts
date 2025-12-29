use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::rwa::RWA;
use crate::fungible::ContractOverrides;
use crate::fungible::FungibleToken;

// functions in RWA trait

#[rule]
// forced_transfer changes balance of from appropriately
// status: timeout
pub fn rwa_forced_transfer_integrity_1(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_from_pre = RWA::balance(&e, &from);
    clog!(balance_from_pre);
    RWA::forced_transfer(&e, &from, &to, amount);
    clog!(cvlr_soroban::Addr(&from));
    let balance_from_post = RWA::balance(&e, &from);
    clog!(balance_from_post);
    if from != to {
        cvlr_assert!(balance_from_post == balance_from_pre - amount);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// forced_transfer changes balance of to appropriately
// status: timeout
pub fn rwa_forced_transfer_integrity_2(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_to_pre = RWA::balance(&e, &to);
    clog!(balance_to_pre);
    RWA::forced_transfer(&e, &from, &to, amount);
    clog!(cvlr_soroban::Addr(&to));
    let balance_to_post = RWA::balance(&e, &to);
    clog!(balance_to_post);
    if from != to {
        cvlr_assert!(balance_to_post == balance_to_pre + amount);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// forced_transfer does not change total supply
// status: verified
pub fn rwa_forced_transfer_integrity_3(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let total_supply_pre = RWA::total_supply(&e);
    clog!(total_supply_pre);
    RWA::forced_transfer(&e, &from, &to, amount);
    let total_supply_post = RWA::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre);
}

// todo: on_transfer hook in compliance contract

#[rule]
// mint increases balance of to appropriately
// status: timeout
pub fn rwa_mint_integrity_1(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_to_pre = RWA::balance(&e, &to);
    clog!(balance_to_pre);
    RWA::mint(&e, &to, amount);
    let balance_to_post = RWA::balance(&e, &to);
    clog!(balance_to_post);
    cvlr_assert!(balance_to_post == balance_to_pre + amount);
}

#[rule]
// mint increases total supply by amount
// status: timeout
pub fn rwa_mint_integrity_2(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let total_supply_pre = RWA::total_supply(&e);
    clog!(total_supply_pre);
    RWA::mint(&e, &to, amount);
    let total_supply_post = RWA::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre + amount);
}

// todo: created hook in compliance contract

#[rule]
// burn decreases balance of user appropriately
// status: timeout
pub fn rwa_burn_integrity_1(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_user_pre = RWA::balance(&e, &user);
    clog!(balance_user_pre);
    RWA::burn(&e, &user, amount);
    let balance_user_post = RWA::balance(&e, &user);
    clog!(balance_user_post);
    cvlr_assert!(balance_user_post == balance_user_pre - amount);
}

#[rule]
// burn decreases total supply by amount
// status: timeout
pub fn rwa_burn_integrity_2(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    let total_supply_pre = RWA::total_supply(&e);
    clog!(total_supply_pre);
    RWA::burn(&e, &user, amount);
    let total_supply_post = RWA::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre - amount);
}

// todo: destroyed hook in compliance contract
// todo: potentially unfreezing tokens

#[rule]
// set_address_frozen sets the frozen status
// status: verified
pub fn rwa_set_address_frozen_integrity(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let freeze_status: bool = nondet();
    clog!(freeze_status);
    let operator = nondet_address();
    RWA::set_address_frozen(&e, &user, freeze_status);
    let frozen_status_post = RWA::is_frozen(&e, &user);
    clog!(frozen_status_post);
    cvlr_assert!(frozen_status_post == freeze_status);
}

#[rule]
// freeze_partial_tokens increase the frozen token amount for a user
// status: verified
pub fn rwa_freeze_partial_tokens_integrity(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    let frozen_tokens_pre = RWA::get_frozen_tokens(&e, &user);
    RWA::freeze_partial_tokens(&e, &user, amount);
    let frozen_tokens_post = RWA::get_frozen_tokens(&e, &user);
    clog!(frozen_tokens_post);
    cvlr_assert!(frozen_tokens_post == frozen_tokens_pre + amount);
}

#[rule]
// unfreeze_partial_tokens decrease the frozen token amount for a user
// status: verified
pub fn rwa_unfreeze_partial_tokens_integrity(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    let frozen_tokens_pre = RWA::get_frozen_tokens(&e, &user);
    clog!(frozen_tokens_pre);
    RWA::unfreeze_partial_tokens(&e, &user, amount);
    let frozen_tokens_post = RWA::get_frozen_tokens(&e, &user);
    clog!(frozen_tokens_post);
    cvlr_assert!(frozen_tokens_post == frozen_tokens_pre - amount);
}

#[rule]
// set_compliance sets the compliance contract
// status: verified
pub fn rwa_set_compliance_integrity(e: Env) {
    let compliance = nondet_address();
    clog!(cvlr_soroban::Addr(&compliance));
    RWA::set_compliance(&e, &compliance);
    let compliance_post = RWA::compliance(&e);
    clog!(cvlr_soroban::Addr(&compliance_post));
    cvlr_assert!(compliance_post == compliance);
}

#[rule]
// set_identity_verifier sets the identity verifier contract
// status: verified
pub fn rwa_set_identity_verifier_integrity(e: Env) {
    let identity_verifier = nondet_address();
    clog!(cvlr_soroban::Addr(&identity_verifier));
    RWA::set_identity_verifier(&e, &identity_verifier);
    let identity_verifier_post = RWA::identity_verifier(&e);
    clog!(cvlr_soroban::Addr(&identity_verifier_post));
    cvlr_assert!(identity_verifier_post == identity_verifier);
}

// functions from the fungible token trait and overriden

#[rule]
// transfer changes balance of from appropriately
// status: timeout
pub fn rwa_transfer_integrity_1(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_from_pre = RWA::balance(&e, &from);
    clog!(balance_from_pre);
    RWA::transfer(&e, &from, &to, amount);
    let balance_from_post = RWA::balance(&e, &from);
    clog!(balance_from_post);
    if from != to {
        cvlr_assert!(balance_from_post == balance_from_pre - amount);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// transfer changes balance of to appropriately
// status: timeout
pub fn rwa_transfer_integrity_2(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_to_pre = RWA::balance(&e, &to);
    clog!(balance_to_pre);
    RWA::transfer(&e, &from, &to, amount);
    let balance_to_post = RWA::balance(&e, &to);
    clog!(balance_to_post);
    if from != to {
        cvlr_assert!(balance_to_post == balance_to_pre + amount);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// transfer does not change total supply
// status: verified
pub fn rwa_transfer_integrity_3(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let total_supply_pre = RWA::total_supply(&e);
    clog!(total_supply_pre);
    RWA::transfer(&e, &from, &to, amount);
    let total_supply_post = RWA::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre);
}

// todo: on_transfer hook in compliance contract

// transfer_from

#[rule]
// transfer_from does not change total supply
// status: timeout
pub fn rwa_transfer_from_integrity_1(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let total_supply_pre = RWA::total_supply(&e);
    clog!(total_supply_pre);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    let total_supply_post = RWA::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post == total_supply_pre);
}

#[rule]
// transfer_from changes the balance of from accordingly
// status: timeout
pub fn rwa_transfer_from_integrity_2(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance_from_pre = RWA::balance(&e, &from);
    clog!(balance_from_pre);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    let balance_from_post = RWA::balance(&e, &from);
    clog!(balance_from_post);
    cvlr_assert!(balance_from_post == balance_from_pre - amount);
}

#[rule]
// transfer_from changes the balance of to accordingly
// status: timeout
pub fn rwa_transfer_from_integrity_3(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    let amount: i128 = nondet();
    clog!(amount);
    let balance_to_pre = RWA::balance(&e, &to);
    clog!(balance_to_pre);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    let balance_to_post = RWA::balance(&e, &to);
    clog!(balance_to_post);
    cvlr_assert!(balance_to_post == balance_to_pre + amount);
}

#[rule]
// transfer_from changes allowance accordingly
// status: timeout
pub fn rwa_transfer_from_integrity_4(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let allowance_pre = RWA::allowance(&e, &from, &spender);
    clog!(allowance_pre);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    let allowance_post = RWA::allowance(&e, &from, &spender);
    clog!(allowance_post);
    cvlr_assert!(allowance_post == allowance_pre - amount);
}

// todo: on_transfer hook in compliance contract

#[rule]
// approve changes allowance accordingly
// status: verified
pub fn rwa_approve_integrity(e: Env) {
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let allowance_pre = RWA::allowance(&e, &owner, &spender);
    clog!(allowance_pre);
    let live_until_ledger: u32 = nondet();
    RWA::approve(&e, &owner, &spender, amount, live_until_ledger);
    let allowance_post = RWA::allowance(&e, &owner, &spender);
    clog!(allowance_post);
    cvlr_assert!(allowance_post == amount);
}

// todo: rules about recover_balance - function that is not exposed in the trait.