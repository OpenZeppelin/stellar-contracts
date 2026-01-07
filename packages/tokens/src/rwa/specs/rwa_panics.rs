use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{is_auth, nondet_address};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use stellar_contract_utils::pausable::paused;

use crate::rwa::RWA;
use crate::fungible::ContractOverrides;
use crate::fungible::FungibleToken;
use crate::rwa::compliance::Compliance;
use crate::rwa::specs::mocks::compliance_trivial::ComplianceTrivial;
use crate::rwa::specs::mocks::identity_verifier_trivial::IdentityVerifierTrivial;

// due to the discrepency between trait function and storage functions
// we cannot verify the fact that authorization of the operator is required
// and other properties on the operator
// this is an issue and should be revised after they change their code.
// unless it is intended that the operator checks are defined by developers

// ============================================================================
// forced_transfer
// ============================================================================

// forced_transfer DOES NOT panic if not compliant
// forced_transfer DOES NOT panic if address if frozen
// forced_transfer DOES NOT panic if contract is paused
// forced_transfer DOES NOT panic if identity verification fails

#[rule]
// forced_transfer panics if not enough balance
// status: verified
pub fn rwa_forced_transfer_panics_if_not_enough_balance(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = RWA::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount);
    RWA::forced_transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// forced_transfer panics if amount < 0
// status: verified
pub fn rwa_forced_transfer_panics_if_amount_less_than_zero(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::forced_transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

// ============================================================================
// mint
// ============================================================================

// mint DOES NOT panic if contract is paused
// mint DOES NOT panic if the recipient address is frozen

pub fn assume_verify_identity_result_map_is_uninit() {
    use crate::rwa::specs::helpers::ghosts::GhostMap::UnInit;
    use crate::rwa::specs::mocks::identity_verifier_trivial::VERIFY_IDENTITY_RESULT_MAP;
    unsafe {
    let verify_identity_result_map: &crate::rwa::specs::helpers::ghosts::GhostMap<Address, bool> = &VERIFY_IDENTITY_RESULT_MAP;
        let is_uninit = verify_identity_result_map.is_uninit();
        clog!(is_uninit);
        cvlr_assume!(is_uninit);
    }
}

pub fn clog_verify_identity_result_map() {
    use crate::rwa::specs::mocks::identity_verifier_trivial::VERIFY_IDENTITY_RESULT_MAP;
    unsafe {
        let verify_identity_result_map: &crate::rwa::specs::helpers::ghosts::GhostMap<Address, bool> = &VERIFY_IDENTITY_RESULT_MAP;
        let is_uninit = verify_identity_result_map.is_uninit();
        clog!(is_uninit);
    }
}

#[rule]
// mint panics if the identity verification fails
// status: spurious violation - review
pub fn rwa_mint_panics_if_identity_verification_fails(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    assume_verify_identity_result_map_is_uninit();
    let verify_identity_result = IdentityVerifierTrivial::verify_identity_non_panicking(&e, &to);
    clog!(verify_identity_result);
    clog_verify_identity_result_map(); // gives uinit = true
    cvlr_assume!(!verify_identity_result);
    RWA::mint(&e, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// mint panics if amount < 0 
// status: verified
pub fn rwa_mint_panics_if_amount_less_than_zero(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::mint(&e, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// mint panics if not compliant
// status: spurious violation - review
pub fn rwa_mint_panics_if_not_compliant(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let can_create = ComplianceTrivial::can_create(&e, to.clone(), amount, e.current_contract_address());
    cvlr_assume!(!can_create);
    RWA::mint(&e, &to, amount);
    cvlr_assert!(false);
}

// ============================================================================
// burn
// ============================================================================

// note:
// burn DOES NOT panic if identity verification fails
// burn DOES NOT panic if contract is paused
// burn DOES NOT panic if the user address is frozen

#[rule]
// burn panics if amount < 0 
// status: verified
pub fn rwa_burn_panics_if_amount_less_than_zero(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::burn(&e, &user, amount);
    cvlr_assert!(false);
}


#[rule]
// burn panics if not enough balance
// status: verified
pub fn rwa_burn_panics_if_not_enough_balance(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = RWA::balance(&e, &user);
    clog!(balance);
    cvlr_assume!(balance < amount);
    RWA::burn(&e, &user, amount);
    cvlr_assert!(false);
}

// ============================================================================
// freeze_partial_tokens
// ============================================================================

#[rule]
// freeze_partial_tokens panics if amount < 0 
// status: verified
pub fn rwa_freeze_partial_tokens_panics_if_amount_less_than_zero(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::freeze_partial_tokens(&e, &user, amount);
    cvlr_assert!(false);
}

// ============================================================================
// unfreeze_partial_tokens
// ============================================================================

#[rule]
// unfreeze_partial_tokens panics if amount < 0 
// status: verified
pub fn rwa_unfreeze_partial_tokens_panics_if_amount_less_than_zero(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::unfreeze_partial_tokens(&e, &user, amount);
    cvlr_assert!(false);
}

#[rule]
// unfreeze_partial_tokens panics if not enough frozen tokens
// status: verified
pub fn rwa_unfreeze_partial_tokens_panics_if_not_enough_frozen_tokens(e: Env) {
    let user = nondet_address();
    clog!(cvlr_soroban::Addr(&user));
    let amount: i128 = nondet();
    clog!(amount);
    let frozen_tokens = RWA::get_frozen_tokens(&e, &user);
    clog!(frozen_tokens);
    cvlr_assume!(frozen_tokens < amount);
    RWA::unfreeze_partial_tokens(&e, &user, amount);
    cvlr_assert!(false);
}

// ============================================================================
// transfer
// ============================================================================

#[rule]
// transfer panics if verification of to fails
// status: spurious violation - review
pub fn rwa_transfer_panics_if_verification_of_to_fails(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let verify_identity_result = IdentityVerifierTrivial::verify_identity_non_panicking(&e, &to);
    clog!(verify_identity_result);
    cvlr_assume!(!verify_identity_result);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if verification of from fails
// status: spurious violation - review
pub fn rwa_transfer_panics_if_verification_of_from_fails(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let verify_identity_result = IdentityVerifierTrivial::verify_identity_non_panicking(&e, &from);
    clog!(verify_identity_result);
    cvlr_assume!(!verify_identity_result);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if amount < 0 
// status: verified
pub fn rwa_transfer_panics_if_amount_less_than_zero(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if from does not auth
// status: verified
pub fn rwa_transfer_panics_if_from_does_not_auth(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(from.clone()));
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if not compliant 
// status: spurious violation - review
pub fn rwa_transfer_panics_if_not_compliant(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let can_transfer = ComplianceTrivial::can_transfer(&e, from.clone(), to.clone(), amount, e.current_contract_address());
    cvlr_assume!(!can_transfer);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if contract is paused
// status: verified
pub fn rwa_transfer_panics_if_contract_paused(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let paused = paused(&e);
    clog!(paused);
    cvlr_assume!(paused);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if the sender address is frozen
// status: verified
pub fn rwa_transfer_panics_if_sender_address_frozen(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let is_frozen = RWA::is_frozen(&e, &from);
    clog!(is_frozen);
    cvlr_assume!(is_frozen);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if the recipient address is frozen
// status: verified
pub fn rwa_transfer_panics_if_recipient_address_frozen(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let is_frozen = RWA::is_frozen(&e, &to);
    clog!(is_frozen);
    cvlr_assume!(is_frozen);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if not enough balance
// status: verified
pub fn rwa_transfer_panics_if_not_enough_balance(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = RWA::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer panics if not enough unfrozen balance
// status: verified
pub fn rwa_transfer_panics_if_not_enough_unfrozen_balance(e: Env) {
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let free_tokens = RWA::get_free_tokens(&e, &from);
    clog!(free_tokens);
    cvlr_assume!(free_tokens < amount);
    RWA::transfer(&e, &from, &to, amount);
    cvlr_assert!(false);
}

// ============================================================================
// transfer_from
// ============================================================================

#[rule]
// transfer_from panics if verification of to fails
// status: spurious violation - review
pub fn rwa_transfer_from_panics_if_verification_of_to_fails(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let verify_identity_result = IdentityVerifierTrivial::verify_identity_non_panicking(&e, &to);
    clog!(verify_identity_result);
    cvlr_assume!(!verify_identity_result);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if verification of from fails
// status: spurious violation - review
pub fn rwa_transfer_from_panics_if_verification_of_from_fails(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let verify_identity_result = IdentityVerifierTrivial::verify_identity_non_panicking(&e, &from);
    clog!(verify_identity_result);
    cvlr_assume!(!verify_identity_result);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if amount < 0 
// status: verified
pub fn rwa_transfer_from_panics_if_amount_less_than_zero(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(amount < 0);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if spender does not auth
// status: bug - transfer_from does not work properly.s
pub fn rwa_transfer_from_panics_if_spender_does_not_auth(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    cvlr_assume!(!is_auth(spender.clone()));
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if not compliant
// status: spurious violation - review
pub fn rwa_transfer_from_panics_if_not_compliant(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let can_transfer = ComplianceTrivial::can_transfer(&e, from.clone(), to.clone(), amount, e.current_contract_address());
    cvlr_assume!(!can_transfer);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if contract is paused
// status: verified
pub fn rwa_transfer_from_panics_if_contract_paused(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let paused = paused(&e);
    clog!(paused);
    cvlr_assume!(paused);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if the sender address is frozen
// status: verified
pub fn rwa_transfer_from_panics_if_sender_address_frozen(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let is_frozen = RWA::is_frozen(&e, &from);
    clog!(is_frozen);
    cvlr_assume!(is_frozen);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if the recipient address is frozen
// status: verified
pub fn rwa_transfer_from_panics_if_recipient_address_frozen(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let is_frozen = RWA::is_frozen(&e, &to);
    clog!(is_frozen);
    cvlr_assume!(is_frozen);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if not enough balance
// status: verified
pub fn rwa_transfer_from_panics_if_not_enough_balance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let balance = RWA::balance(&e, &from);
    clog!(balance);
    cvlr_assume!(balance < amount); 
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}

#[rule]
// transfer_from panics if not enough unfrozen balance
// status: verified
pub fn rwa_transfer_from_panics_if_not_enough_unfrozen_balance(e: Env) {
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let free_tokens = RWA::get_free_tokens(&e, &from);
    clog!(free_tokens);
    cvlr_assume!(free_tokens < amount);
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_assert!(false);
}