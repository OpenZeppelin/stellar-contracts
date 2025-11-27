use cvlr::{cvlr_assert, cvlr_assume,cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_symbol};
use cvlr::nondet::Nondet;
use cvlr_soroban_derive::rule;
use cvlr::clog;

use soroban_sdk::{Env, Address, Symbol};
use crate::access_control::{AccessControl, specs::{access_control_contract::AccessControlContract, helper::{get_pending_admin, get_role_account}}};
use crate::access_control::specs::constructor_helper::{before_constructor_no_has_role, before_constructor_no_role_accounts, before_constructor_no_role_count};

// invariant: admin != None -> holds in all cases except for renounce_admin

// helpers
pub fn assume_pre_admin_is_set(e: Env) {
    let admin_pre = AccessControlContract::get_admin(&e);
    cvlr_assume!(admin_pre.is_some());
}

pub fn assert_post_admin_is_set(e: Env) {
    let admin_post = AccessControlContract::get_admin(&e);
    cvlr_assert!(admin_post.is_some());
}

#[rule]
// status: verified 
pub fn after_constructor_admin_is_set(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_constructor_admin_is_set_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_grant_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_grant_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_revoke_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_revoke_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_renounce_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_renounce_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_set_role_admin_admin_is_set(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    assert_post_admin_is_set(e);
}

#[rule]
// status: verified
pub fn after_set_role_admin_admin_is_set_sanity(e: Env) {
    assume_pre_admin_is_set(e.clone());
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    cvlr_satisfy!(true);
}

// for the case renonuce_admin it's obviously true - and expected

// invariant: pending_admin != none implies admin != none

// helpers
pub fn assume_pre_pending_admin_implies_admin(e: &Env) {
    let pending_admin_pre = get_pending_admin(&e);
    let admin = AccessControlContract::get_admin(&e);
    if pending_admin_pre.is_some() {
        cvlr_assume!(admin.is_some());
    }
}

pub fn assert_post_pending_admin_implies_admin(e: &Env) {
    let pending_admin_post = get_pending_admin(&e);
    let admin = AccessControlContract::get_admin(&e);
    if pending_admin_post.is_some() {
        cvlr_assert!(admin.is_some());
    }
}

#[rule]
// status: verified
pub fn after_constructor_pending_admin_implies_admin(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_constructor_pending_admin_implies_admin_sanity(e: Env) {
    let admin = nondet_address();
    AccessControlContract::access_control_constructor(&e, admin);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_grant_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_grant_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_revoke_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_revoke_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let account = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::revoke_role(&e, caller, account, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_renounce_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_renounce_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let caller = nondet_address();
    let role = nondet_symbol();
    AccessControlContract::renounce_role(&e, caller, role);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let new_admin = nondet_address();
    let live_until_ledger = u32::nondet();
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::accept_admin_transfer(&e);
    cvlr_satisfy!(true);
}

#[rule]
// status: verified
pub fn after_set_role_admin_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_set_role_admin_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    let role = nondet_symbol();
    let admin_role = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role, admin_role);
    cvlr_satisfy!(true);
}

#[rule]
// status: violated - bug
pub fn after_renounce_admin_pending_admin_implies_admin(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::renounce_admin(&e);
    assert_post_pending_admin_implies_admin(&e);
}

#[rule]
// status: verified
pub fn after_renounce_admin_pending_admin_implies_admin_sanity(e: Env) {
    assume_pre_pending_admin_implies_admin(&e);
    AccessControlContract::renounce_admin(&e);
    cvlr_satisfy!(true);
}

// invariant: the index of two different accounts with the same role is different

// helpers
pub fn assume_pre_unique_indices_for_role(
    e: &Env, account1: Address, account2: Address, role: Symbol
) {
    clog!(cvlr_soroban::Addr(&account1));
    clog!(cvlr_soroban::Addr(&account2));
    let index1 = AccessControlContract::has_role(&e, account1.clone(), role.clone());
    let index2 = AccessControlContract::has_role(&e, account2.clone(), role.clone());
    clog!(index1);
    clog!(index2);
    if index1.is_some() && index2.is_some() && account1 != account2 {
        cvlr_assume!(index1.unwrap() != index2.unwrap());
    }
}

pub fn assert_post_unique_indices_for_role(
    e: &Env, account1: Address, account2: Address, role: Symbol
) {
    clog!(cvlr_soroban::Addr(&account1));
    clog!(cvlr_soroban::Addr(&account2));
    let index1 = AccessControlContract::has_role(&e, account1.clone(), role.clone());
    let index2 = AccessControlContract::has_role(&e, account2.clone(), role.clone());
    clog!(index1);
    clog!(index2);
    if index1.is_some() && index2.is_some() && account1 != account2 {
        cvlr_assert!(index1.unwrap() != index2.unwrap());
    }
}

// didn't do sanity for these.

#[rule]
// status: verified
pub fn after_constructor_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    before_constructor_no_has_role(&e, account1.clone(), role.clone());
    before_constructor_no_has_role(&e, account2.clone(), role.clone());
    let admin: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&admin));
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_grant_role_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    assume_pre_role_count_minus_one_geq_index(&e, account1.clone(), role.clone());
    assume_pre_role_count_minus_one_geq_index(&e, account2.clone(), role.clone());
    let caller: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    let role_granted = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account, role_granted);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_revoke_role_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    let caller: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    let role_revoked = nondet_symbol();
    assume_pre_unique_indices_for_role(&e, account1.clone(), account.clone(), role.clone());
    assume_pre_unique_indices_for_role(&e, account2.clone(), account.clone(), role.clone());
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    AccessControlContract::revoke_role(&e, caller, account, role_revoked);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_renounce_role_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let role_renounced = nondet_symbol();
    assume_pre_unique_indices_for_role(&e, account1.clone(), caller.clone(), role.clone());
    assume_pre_unique_indices_for_role(&e, account2.clone(), caller.clone(), role.clone());
    assume_pre_role_count_minus_one_geq_index(&e, caller.clone(), role.clone());
    AccessControlContract::renounce_role(&e, caller, role_renounced);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    let new_admin = nondet_address();
    clog!(cvlr_soroban::Addr(&new_admin));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_set_role_admin_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    let role_admin = nondet_symbol();
    let role_treated = nondet_symbol(); 
    AccessControlContract::set_role_admin(&e, role_treated, role_admin);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_renounce_admin_unique_indices_for_role(
    e: Env, account1: Address, account2: Address, role: Symbol
) {
    assume_pre_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
    AccessControlContract::renounce_admin(&e);
    assert_post_unique_indices_for_role(&e, account1.clone(), account2.clone(), role.clone());
}

// invariant role_count - 1 >= has_role(address) (for any address)

// helpers
pub fn assume_pre_role_count_minus_one_geq_index(
    e: &Env, account: Address, role: Symbol
) {
    clog!(cvlr_soroban::Addr(&account));
    let role_count = AccessControlContract::get_role_member_count(&e, role.clone());
    clog!(role_count);
    let index = AccessControlContract::has_role(&e, account.clone(), role.clone());
    clog!(index);
    if index.is_some() {
        cvlr_assume!(role_count - 1 >= index.unwrap());
    }
}

pub fn assert_post_role_count_minus_one_geq_index(
    e: &Env, account: Address, role: Symbol
) {
    clog!(cvlr_soroban::Addr(&account));
    let role_count = AccessControlContract::get_role_member_count(&e, role.clone());
    clog!(role_count);
    let index = AccessControlContract::has_role(&e, account.clone(), role.clone());
    clog!(index);
    if index.is_some() {
        cvlr_assert!(role_count - 1 >= index.unwrap());
    }
}

#[rule]
// status: verified
pub fn after_constructor_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    before_constructor_no_has_role(&e, account.clone(), role.clone());
    before_constructor_no_role_count(&e, &role);
    let admin = nondet_address();
    clog!(cvlr_soroban::Addr(&admin));
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_grant_role_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account_granted = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    let role_granted = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account_granted, role_granted);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: violated - spurious (i think)
// see below for renounce_role
pub fn after_revoke_role_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account_revoked = nondet_address();
    clog!(cvlr_soroban::Addr(&account_revoked));
    let role_revoked = nondet_symbol();
    assume_pre_role_count_minus_one_geq_index(&e, account_revoked.clone(), role_revoked.clone()); // like requireInvariant in CVL
    AccessControlContract::revoke_role(&e, caller, account_revoked, role_revoked);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: violated - not sure
// https://prover.certora.com/output/5771024/b3daf131f5ea41d69ebbd2684ce3520b/?anonymousKey=94c303fa729eabe38df7e5ffca4f30e736a2aba2&params=%7B%2225%22%3A%7B%22index%22%3A0%2C%22ruleCounterExamples%22%3A%5B%7B%22name%22%3A%22rule_output_19.json%22%2C%22selectedRepresentation%22%3A%7B%22label%22%3A%22PRETTY%22%2C%22value%22%3A0%7D%2C%22callResolutionSingleFilter%22%3A%22%22%2C%22variablesFilter%22%3A%22%22%2C%22callTraceFilter%22%3A%22%22%2C%22variablesOpenItems%22%3A%5Btrue%2Ctrue%5D%2C%22callTraceCollapsed%22%3Atrue%2C%22rightSidePanelCollapsed%22%3Afalse%2C%22rightSideTab%22%3A%22%22%2C%22callResolutionSingleCollapsed%22%3Atrue%2C%22viewStorage%22%3Atrue%2C%22variablesExpandedArray%22%3A%22%22%2C%22expandedArray%22%3A%22509-10-12-186-1-1-1248-1-1319_320-1360_361-1-1508%22%2C%22orderVars%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22orderParams%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22scrollNode%22%3A%2288%22%2C%22currentPoint%22%3A0%2C%22trackingChildren%22%3A%5B%5D%2C%22trackingParents%22%3A%5B%5D%2C%22trackingOnly%22%3Afalse%2C%22highlightOnly%22%3Afalse%2C%22filterPosition%22%3A0%2C%22singleCallResolutionOpen%22%3A%5B%5D%2C%22snap_drop_1%22%3Anull%2C%22snap_drop_2%22%3Anull%2C%22snap_filter%22%3A%22%22%7D%5D%7D%7D&generalState=%7B%22fileViewOpen%22%3Afalse%2C%22fileViewCollapsed%22%3Atrue%2C%22mainTreeViewCollapsed%22%3Atrue%2C%22callTraceClosed%22%3Afalse%2C%22mainSideNavItem%22%3A%22rules%22%2C%22globalResSelected%22%3Afalse%2C%22isSideBarCollapsed%22%3Afalse%2C%22isRightSideBarCollapsed%22%3Atrue%2C%22selectedFile%22%3A%7B%7D%2C%22fileViewFilter%22%3A%22%22%2C%22mainTreeViewFilter%22%3A%22%22%2C%22contractsFilter%22%3A%22%22%2C%22globalCallResolutionFilter%22%3A%22%22%2C%22currentRuleUiId%22%3A25%2C%22counterExamplePos%22%3A1%2C%22expandedKeysState%22%3A%2224-10-1-1-1-1-1-116-118-123-1-1%22%2C%22expandedFilesState%22%3A%5B%5D%2C%22outlinedfilterShared%22%3A%22000000000%22%7D
pub fn after_renounce_role_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let role_renounced = nondet_symbol();
    assume_pre_role_count_minus_one_geq_index(&e, caller.clone(), role_renounced.clone()); // like requireInvariant in CVL
    assume_pre_unique_indices_for_role(&e, caller.clone(), account.clone(), role.clone());
    assume_pre_has_role_index_implies_get_role_account(&e, caller.clone(), role.clone());
    AccessControlContract::renounce_role(&e, caller, role_renounced);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    let new_admin = nondet_address();
    clog!(cvlr_soroban::Addr(&new_admin));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_set_role_admin_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    let role_admin = nondet_symbol();
    let role_treated = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role_treated, role_admin);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_renounce_admin_role_count_minus_one_geq_index(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
    AccessControlContract::renounce_admin(&e);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

// you would also want Exists(address). has_role(address) = role_count - 1 but not supported.

// invariant: if has_role(account,role) = index then get_account_role(role,index) = account

// helpers
pub fn assume_pre_has_role_index_implies_get_role_account(
    e: &Env, account: Address, role: Symbol
) {
    clog!(cvlr_soroban::Addr(&account));
    let index = AccessControlContract::has_role(&e, account.clone(), role.clone());
    clog!(index);
    if index.is_some() {
        let account_with_index = get_role_account(&e, &role, index.unwrap());
        if let Some(account_with_index_internal) = account_with_index.clone() {
            clog!(cvlr_soroban::Addr(&account_with_index_internal));
        }
        cvlr_assume!(account_with_index == Some(account));
    }
}

pub fn assert_post_has_role_index_implies_get_role_account(
    e: &Env, account: Address, role: Symbol
) {
    clog!(cvlr_soroban::Addr(&account));
    let index = AccessControlContract::has_role(&e, account.clone(), role.clone());
    clog!(index);
    if index.is_some() {
        let account_with_index = get_role_account(&e, &role, index.unwrap());
        if let Some(account_with_index_internal) = account_with_index.clone() {
            clog!(cvlr_soroban::Addr(&account_with_index_internal));
        }
        cvlr_assert!(account_with_index == Some(account));
    }
}


#[rule]
// status: verified
pub fn after_constructor_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    before_constructor_no_has_role(&e, account.clone(), role.clone());
    before_constructor_no_role_accounts(&e, role.clone(), 0);
    let admin = nondet_address();
    clog!(cvlr_soroban::Addr(&admin));
    AccessControlContract::access_control_constructor(&e, admin);
    assert_post_role_count_minus_one_geq_index(&e, account.clone(), role.clone());
}

#[rule]
// status: spurious - prover bug ? Some(0) = Some(11) https://prover.certora.com/output/5771024/6e270c43416b4137b3fc221758f6cf47/?anonymousKey=cc464cb85345dd06596fdecb09d7f7414369e434&params=%7B%2212%22%3A%7B%22index%22%3A0%2C%22ruleCounterExamples%22%3A%5B%7B%22name%22%3A%22rule_output_3.json%22%2C%22selectedRepresentation%22%3A%7B%22label%22%3A%22PRETTY%22%2C%22value%22%3A0%7D%2C%22callResolutionSingleFilter%22%3A%22%22%2C%22variablesFilter%22%3A%22%22%2C%22callTraceFilter%22%3A%22%22%2C%22variablesOpenItems%22%3A%5Btrue%2Ctrue%5D%2C%22callTraceCollapsed%22%3Atrue%2C%22rightSidePanelCollapsed%22%3Afalse%2C%22rightSideTab%22%3A%22%22%2C%22callResolutionSingleCollapsed%22%3Atrue%2C%22viewStorage%22%3Atrue%2C%22variablesExpandedArray%22%3A%22%22%2C%22expandedArray%22%3A%22208-10-12-1-1-1-1-1-1-1-1-1207%22%2C%22orderVars%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22orderParams%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22scrollNode%22%3A%221%22%2C%22currentPoint%22%3A0%2C%22trackingChildren%22%3A%5B%5D%2C%22trackingParents%22%3A%5B%5D%2C%22trackingOnly%22%3Afalse%2C%22highlightOnly%22%3Afalse%2C%22filterPosition%22%3A0%2C%22singleCallResolutionOpen%22%3A%5B%5D%2C%22snap_drop_1%22%3Anull%2C%22snap_drop_2%22%3Anull%2C%22snap_filter%22%3A%22%22%7D%5D%7D%7D&generalState=%7B%22fileViewOpen%22%3Afalse%2C%22fileViewCollapsed%22%3Atrue%2C%22mainTreeViewCollapsed%22%3Atrue%2C%22callTraceClosed%22%3Afalse%2C%22mainSideNavItem%22%3A%22rules%22%2C%22globalResSelected%22%3Afalse%2C%22isSideBarCollapsed%22%3Afalse%2C%22isRightSideBarCollapsed%22%3Atrue%2C%22selectedFile%22%3A%7B%7D%2C%22fileViewFilter%22%3A%22%22%2C%22mainTreeViewFilter%22%3A%22%22%2C%22contractsFilter%22%3A%22%22%2C%22globalCallResolutionFilter%22%3A%22%22%2C%22currentRuleUiId%22%3A12%2C%22counterExamplePos%22%3A1%2C%22expandedKeysState%22%3A%228-10-1-02-03-04-1-1-1-08-1-1%22%2C%22expandedFilesState%22%3A%5B%5D%2C%22outlinedfilterShared%22%3A%22000000000%22%7D
pub fn after_grant_role_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account_granted = nondet_address();
    clog!(cvlr_soroban::Addr(&account));
    let role_granted = nondet_symbol();
    AccessControlContract::grant_role(&e, caller, account_granted, role_granted);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
// 9 minute timeout
pub fn after_revoke_role_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let account_revoked = nondet_address();
    clog!(cvlr_soroban::Addr(&account_revoked));
    let role_revoked = nondet_symbol();
    assume_pre_unique_indices_for_role(&e, account.clone(), account_revoked.clone(), role.clone()); // like requireInvariant in CVL
    AccessControlContract::revoke_role(&e, caller, account_revoked, role_revoked);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_renounce_role_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    let caller = nondet_address();
    clog!(cvlr_soroban::Addr(&caller));
    let role_renounced = nondet_symbol();
    assume_pre_unique_indices_for_role(&e, account.clone(), caller.clone(), role.clone()); // like requireInvariant in CVL
    AccessControlContract::renounce_role(&e, caller, role_renounced);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_transfer_admin_role_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    let new_admin = nondet_address();
    clog!(cvlr_soroban::Addr(&new_admin));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    AccessControlContract::transfer_admin_role(&e, new_admin, live_until_ledger);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_accept_admin_transfer_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    AccessControlContract::accept_admin_transfer(&e);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_set_role_admin_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    let role_admin = nondet_symbol();
    let role_treated = nondet_symbol();
    AccessControlContract::set_role_admin(&e, role_treated, role_admin);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}

#[rule]
// status: verified
pub fn after_renounce_admin_has_role_index_implies_get_role_account(
    e: Env, account: Address, role: Symbol
) {
    assume_pre_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
    AccessControlContract::renounce_admin(&e);
    assert_post_has_role_index_implies_get_role_account(&e, account.clone(), role.clone());
}
