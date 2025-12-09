use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    extensions::royalties::storage::{NFTRoyaltiesStorageKey, RoyaltyInfo},
    specs::helper::{is_approved_for_token, is_owned},
    Base,
};

// helpers

pub fn get_default_royalty(e: &Env) -> Option<RoyaltyInfo> {
    let key = NFTRoyaltiesStorageKey::DefaultRoyalty;
    let royalty_info = e.storage().instance().get::<_, RoyaltyInfo>(&key);
    royalty_info
}

pub fn get_token_royalty(e: &Env, token_id: u32) -> Option<RoyaltyInfo> {
    let key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
    let royalty_info = e.storage().persistent().get::<_, RoyaltyInfo>(&key);
    royalty_info
}

// ################## INTEGRITY RULES ##################

#[rule]
// after set_default_royalty the default royalty is set
// status: verified
pub fn set_default_royalty_integrity(e: Env) {
    let receiver = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let basis_points = u32::nondet();
    clog!(basis_points);
    Base::set_default_royalty(&e, &receiver, basis_points);
    let default_royalty = get_default_royalty(&e);
    cvlr_assert!(default_royalty.is_some());
    if let Some(default_royalty_internal) = default_royalty {
        let default_royalty_receiver = default_royalty_internal.receiver;
        let default_royalty_basis_points = default_royalty_internal.basis_points;
        clog!(cvlr_soroban::Addr(&default_royalty_receiver));
        clog!(default_royalty_basis_points);
        cvlr_assert!(default_royalty_receiver == receiver);
        cvlr_assert!(default_royalty_basis_points == basis_points);
    }
}

#[rule]
// after set_token_royalty the token royalty is set
// status: verified
pub fn set_token_royalty_integrity(e: Env) {
    let token_id = u32::nondet();
    clog!(token_id);
    let receiver = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let basis_points = u32::nondet();
    clog!(basis_points);
    Base::set_token_royalty(&e, token_id, &receiver, basis_points);
    let token_royalty = get_token_royalty(&e, token_id);
    cvlr_assert!(token_royalty.is_some());
    if let Some(token_royalty_internal) = token_royalty {
        let token_royalty_receiver = token_royalty_internal.receiver;
        let token_royalty_basis_points = token_royalty_internal.basis_points;
        clog!(cvlr_soroban::Addr(&token_royalty_receiver));
        clog!(token_royalty_basis_points);
        cvlr_assert!(token_royalty_receiver == receiver);
        cvlr_assert!(token_royalty_basis_points == basis_points);
    }
}

#[rule]
// after remove_token_royalty the token royalty is the default
// status: verified
pub fn remove_token_royalty_integrity(e: Env) {
    let token_id = u32::nondet();
    clog!(token_id);
    Base::remove_token_royalty(&e, token_id);
    let token_royalty = get_token_royalty(&e, token_id);
    cvlr_assert!(token_royalty.is_none());
}

// rules that show royalty is calculated correctly

#[rule]
// if there is a specific token_royalty - that is used
// status: verified
pub fn royalty_info_token_royalty_is_some(e: Env) {
    let token_id = u32::nondet();
    clog!(token_id);
    let royalty_info = get_token_royalty(&e, token_id).unwrap(); // assume there is a token royalty
    let royalty_info_receiver = royalty_info.receiver;
    let royalty_info_basis_points = royalty_info.basis_points;
    clog!(cvlr_soroban::Addr(&royalty_info_receiver));
    clog!(royalty_info_basis_points);
    let sale_price: i128 = i128::nondet();
    clog!(sale_price);
    let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, sale_price);
    clog!(cvlr_soroban::Addr(&royalty_receiver));
    clog!(royalty_amount);
    cvlr_assert!(royalty_receiver == royalty_info_receiver);
    cvlr_assert!(royalty_amount == sale_price * royalty_info_basis_points as i128 / 10000);
}

#[rule]
// if there is no specific token royalty, but there is a default, the royalty is
// the default status: verified
pub fn royalty_info_token_royalty_is_none_and_default_is_some(e: Env) {
    let token_id = u32::nondet();
    clog!(token_id);
    let specific_royalty = get_token_royalty(&e, token_id);
    cvlr_assume!(specific_royalty.is_none());
    let default_royalty = get_default_royalty(&e).unwrap(); // assume there is a default royalty
    let default_royalty_receiver = default_royalty.receiver;
    let default_royalty_basis_points = default_royalty.basis_points;
    clog!(cvlr_soroban::Addr(&default_royalty_receiver));
    clog!(default_royalty_basis_points);
    let sale_price: i128 = i128::nondet();
    clog!(sale_price);
    let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, sale_price);
    clog!(cvlr_soroban::Addr(&royalty_receiver));
    clog!(royalty_amount);
    cvlr_assert!(royalty_receiver == default_royalty_receiver);
    cvlr_assert!(royalty_amount == sale_price * default_royalty_basis_points as i128 / 10000);
}

#[rule]
// if there is no specific token royalty, and no default, the royalty is 0 and
// receiver is the contract address status: verified
pub fn royalty_info_token_royalty_is_none_and_default_is_none(e: Env) {
    let token_id = u32::nondet();
    clog!(token_id);
    let specific_royalty = get_token_royalty(&e, token_id);
    cvlr_assume!(specific_royalty.is_none());
    let default_royalty = get_default_royalty(&e);
    cvlr_assume!(default_royalty.is_none());
    let sale_price: i128 = i128::nondet();
    clog!(sale_price);
    let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, sale_price);
    clog!(cvlr_soroban::Addr(&royalty_receiver));
    clog!(royalty_amount);
    cvlr_assert!(royalty_receiver == e.current_contract_address());
    cvlr_assert!(royalty_amount == 0);
}
