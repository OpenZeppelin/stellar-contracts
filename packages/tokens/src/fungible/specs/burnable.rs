use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{is_auth, nondet_address};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{
    specs::{
        fungible_non_panics::{storage_setup_allowance, storage_setup_balance},
    },
    Base,
};

// ################## INTEGRITY RULES ##################

#[rule]
// after burn the account's balance and total supply decrease by amount
// status: verified
// note: 20 minutes
pub fn burn_integrity(e: Env) {
    let account = nondet_address();
    let amount = nondet();
    let balance_pre = Base::balance(&e, &account);
    let total_supply_pre = Base::total_supply(&e);
    Base::burn(&e, &account, amount);
    let balance_post = Base::balance(&e, &account);
    let total_supply_post = Base::total_supply(&e);
    cvlr_assert!(balance_post == balance_pre - amount);
    cvlr_assert!(total_supply_post == total_supply_pre - amount);
}

#[rule]
// after burn_from the total supply decrease by amount
// status: verified
// note: 21 min
pub fn burn_from_integrity_1(e: Env) {
    let account = nondet_address();
    let amount = nondet();
    let total_supply_pre = Base::total_supply(&e);
    Base::burn_from(&e, &account, &account, amount);
    let total_supply_post = Base::total_supply(&e);
    cvlr_assert!(total_supply_post == total_supply_pre - amount);
}

#[rule]
// after burn_from the account's balance decrease by amount
// status: verified
// note: 15 min
pub fn burn_from_integrity_2(e: Env) {
    let account = nondet_address();
    let amount = nondet();
    let balance_pre = Base::balance(&e, &account);
    Base::burn_from(&e, &account, &account, amount);
    let balance_post = Base::balance(&e, &account);
    cvlr_assert!(balance_post == balance_pre - amount);
}

// ################## PANIC RULES ##################

#[rule]
// burn panics if not auth by from
// status: verified
pub fn burn_panics_if_unauthorized(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(from.clone()));
    Base::burn(&e, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn panics if not enough balance
// status: verified
pub fn burn_panics_if_not_enough_balance(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = Base::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount);
    Base::burn(&e, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn panics if amount < 0
// status: verified
pub fn burn_panics_if_amount_less_than_zero(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    Base::burn(&e, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if not auth by spender
// status: verified
pub fn burn_from_panics_if_spender_unauthorized(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(spender.clone()));
    Base::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if not enough balance
// status: verified
pub fn burn_from_panics_if_not_enough_balance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = Base::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount);
    Base::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if not enough allowance
// status: bug
// same bug as in transfer_from
pub fn burn_from_panics_if_not_enough_allowance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    let allowance = Base::allowance(&e, &from, &spender);
    clog!(allowance);
    cvlr_assume!(allowance < amount);
    Base::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(false);
}

#[rule]
// burn_from panics if amount < 0
// status: verified
pub fn burn_from_panics_if_amount_less_than_zero(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    Base::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(false);
}

// ################## NON-PANIC RULES ##################

#[rule]
// requires
// from auth
// from has enough balance
// amount >= 0
// status: wip
pub fn burn_non_panic(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(from.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let from_balance = Base::balance(&e, &from);
    clog!(from_balance);
    cvlr_assume!(from_balance >= amount);
    cvlr_assume!(amount >= 0);
    Base::burn(&e, &from, amount);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: wip
pub fn burn_non_panic_sanity(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(from.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let from_balance = Base::balance(&e, &from);
    clog!(from_balance);
    cvlr_assume!(from_balance >= amount);
    cvlr_assume!(amount >= 0);
    Base::burn(&e, &from, amount);
    cvlr_satisfy!(true);
}

#[rule]
// requires
// spender auth
// from has enough balance
// from has enough allowance
// amount >= 0
// status: wip
pub fn burn_from_non_panic(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(spender.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let balance_from = Base::balance(&e, &from);
    clog!(balance_from);
    cvlr_assume!(balance_from >= amount);
    storage_setup_allowance(e.clone(), from.clone(), spender.clone());
    let allowance_spender = Base::allowance(&e, &from, &spender);
    clog!(allowance_spender);
    cvlr_assume!(allowance_spender >= amount);
    cvlr_assume!(amount >= 0);
    Base::burn_from(&e, &spender, &from, amount);
    cvlr_assert!(true);
}

#[rule]
// sanity
// status: wip
pub fn burn_from_non_panic_sanity(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(is_auth(spender.clone()));
    storage_setup_balance(e.clone(), from.clone());
    let balance_from = Base::balance(&e, &from);
    clog!(balance_from);
    cvlr_assume!(balance_from >= amount);
    storage_setup_allowance(e.clone(), from.clone(), spender.clone());
    let allowance_spender = Base::allowance(&e, &from, &spender);
    clog!(allowance_spender);
    cvlr_assume!(allowance_spender >= amount);
    cvlr_assume!(amount >= 0);
    Base::burn_from(&e, &spender, &from, amount);
    cvlr_satisfy!(true);
}

// ################## INVARIANT RULES ##################

// again, we would like to show total_supply >= balance, but that requires ghosts and hooks.
