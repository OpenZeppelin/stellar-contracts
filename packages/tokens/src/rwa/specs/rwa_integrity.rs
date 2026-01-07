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
// status: verified https://prover.certora.com/output/33158/82f91444a64247649c1ea229fc9eb2c3
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
// status: verified https://prover.certora.com/output/33158/e2033d4a0977480c9ed77f796bc441a9
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
// status: verified 
// https://prover.certora.com/output/33158/824613f78bed403fafa3cc198244524b
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
// status: verified
// https://prover.certora.com/output/33158/311d6caa88244ce49a62e39308d278f4
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
// status: verified
// https://prover.certora.com/output/33158/5d4db829fe6f4c6fa97d1e24d46bfb96
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
// status: verified
// https://prover.certora.com/output/33158/d20d7948e52e4e3f80e063f752943316
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
// status: verified
// https://prover.certora.com/output/33158/7166d4a5413a42b69c0a77d3893967a8
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
// status: verified
// https://prover.certora.com/output/33158/e628639846c44e87a18f6d1a9996b741
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
// status: verified
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
    if from != to {
        cvlr_assert!(balance_from_post == balance_from_pre - amount);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// transfer_from changes the balance of to accordingly
// status: timeout (with -split false)
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
    if from != to {
        cvlr_assert!(balance_to_post == balance_to_pre + amount);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// transfer_from changes allowance accordingly
// status: bug
// https://prover.certora.com/output/33158/61aa0d179f40419889d3857850770761
// same bug as the bug in fungible
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

#[rule]
// output of recover_balance equals balance > 0
// status: verified
pub fn rwa_recover_balance_integrity_1(e: Env) {
    let old_account = nondet_address();
    clog!(cvlr_soroban::Addr(&old_account));
    let new_account = nondet_address();
    clog!(cvlr_soroban::Addr(&new_account));
    let balance_old_account_pre = RWA::balance(&e, &old_account);
    clog!(balance_old_account_pre);
    let balance_was_non_zero = balance_old_account_pre > 0;
    clog!(balance_was_non_zero);
    let balance_new_account_pre = RWA::balance(&e, &new_account);
    clog!(balance_new_account_pre);
    let output = RWA::recover_balance(&e, &old_account, &new_account);
    let balance_new_account_post = RWA::balance(&e, &new_account);
    clog!(balance_new_account_post);
    let balance_old_account_post = RWA::balance(&e, &old_account);
    clog!(balance_old_account_post);
    clog!(output);
    cvlr_assert!(output == balance_was_non_zero);
}

#[rule]
// after recover_balance the old account has no balance
// status: verified
// https://prover.certora.com/output/33158/5103c07236b24a98aa4314b95444651c
pub fn rwa_recover_balance_integrity_2(e: Env) {
    let old_account = nondet_address();
    clog!(cvlr_soroban::Addr(&old_account));
    let new_account = nondet_address();
    clog!(cvlr_soroban::Addr(&new_account));
    let balance_old_account_pre = RWA::balance(&e, &old_account);
    clog!(balance_old_account_pre);
    let balance_new_account_pre = RWA::balance(&e, &new_account);
    clog!(balance_new_account_pre);
    let output = RWA::recover_balance(&e, &old_account, &new_account);
    clog!(output);
    let balance_new_account_post = RWA::balance(&e, &new_account);
    clog!(balance_new_account_post);
    let balance_old_account_post = RWA::balance(&e, &old_account);
    clog!(balance_old_account_post);
    if output {
        if new_account != old_account {
            cvlr_assert!(balance_old_account_post == 0);
        } else {
            cvlr_assert!(balance_old_account_post == balance_old_account_pre);
        }
    } else {
        cvlr_assert!(balance_old_account_post == balance_old_account_pre);
    }
}

#[rule]
// after recover_balance the new account has the balance of the old account
// status: verified
// https://prover.certora.com/output/33158/655e283772ab43e494f7599bb5a256a2
pub fn rwa_recover_balance_integrity_3(e: Env) {
    let old_account = nondet_address();
    clog!(cvlr_soroban::Addr(&old_account));
    let new_account = nondet_address();
    clog!(cvlr_soroban::Addr(&new_account));
    let balance_old_account_pre = RWA::balance(&e, &old_account);
    clog!(balance_old_account_pre);
    let balance_new_account_pre = RWA::balance(&e, &new_account);
    clog!(balance_new_account_pre);
    let output = RWA::recover_balance(&e, &old_account, &new_account);
    clog!(output);
    let balance_new_account_post = RWA::balance(&e, &new_account);
    clog!(balance_new_account_post);
    let balance_old_account_post = RWA::balance(&e, &old_account);
    clog!(balance_old_account_post);
    if output {
        if old_account != new_account {
            cvlr_assert!(balance_new_account_post == balance_old_account_pre + balance_new_account_pre);
        } else {
            cvlr_assert!(balance_new_account_post == balance_old_account_pre);
        }
    } else {
        cvlr_assert!(balance_new_account_post == balance_new_account_pre);
    }
}


#[rule]
// after recover_balance the new account has the frozen tokens of the old account
// status: verified
// https://prover.certora.com/output/33158/5336dbb94c6f4f77a83ae09c9be3c031
pub fn rwa_recover_balance_integrity_4(e: Env) {
    let old_account = nondet_address();
    clog!(cvlr_soroban::Addr(&old_account));
    let new_account = nondet_address();
    clog!(cvlr_soroban::Addr(&new_account));
    let old_account_frozen_tokens_pre = RWA::get_frozen_tokens(&e, &old_account);
    clog!(old_account_frozen_tokens_pre);
    let new_account_frozen_tokens_pre = RWA::get_frozen_tokens(&e, &new_account);
    clog!(new_account_frozen_tokens_pre);
    cvlr_assume!(old_account_frozen_tokens_pre >= 0);
    cvlr_assume!(new_account_frozen_tokens_pre >= 0);
    let output = RWA::recover_balance(&e, &old_account, &new_account);
    clog!(output);
    let new_account_frozen_tokens_post = RWA::get_frozen_tokens(&e, &new_account);
    clog!(new_account_frozen_tokens_post);
    let old_account_frozen_tokens_post = RWA::get_frozen_tokens(&e, &old_account);
    clog!(old_account_frozen_tokens_post);
    if output {
        cvlr_assert!(new_account_frozen_tokens_post >= old_account_frozen_tokens_pre);
    } else {
        cvlr_assert!(new_account_frozen_tokens_post == new_account_frozen_tokens_pre);
        cvlr_assert!(old_account_frozen_tokens_post == old_account_frozen_tokens_pre);
    }
}

#[rule]
// after recover_balance the new account has the frozen status of the old account
// status: verified
pub fn rwa_recover_balance_integrity_5(e: Env) {
    let old_account = nondet_address();
    clog!(cvlr_soroban::Addr(&old_account));
    let new_account = nondet_address();
    clog!(cvlr_soroban::Addr(&new_account));
    let old_account_frozen_status_pre = RWA::is_frozen(&e, &old_account);
    clog!(old_account_frozen_status_pre);
    let new_account_frozen_status_pre = RWA::is_frozen(&e, &new_account);
    clog!(new_account_frozen_status_pre);
    let output = RWA::recover_balance(&e, &old_account, &new_account);
    clog!(output);
    let new_account_frozen_status_post = RWA::is_frozen(&e, &new_account);
    clog!(new_account_frozen_status_post);
    let old_account_frozen_status_post = RWA::is_frozen(&e, &old_account);
    clog!(old_account_frozen_status_post);
    if output {
        if old_account_frozen_status_pre {
            cvlr_assert!(new_account_frozen_status_post);
        } else {
            cvlr_assert!(new_account_frozen_status_post == new_account_frozen_status_pre);
        }
    } else {
        cvlr_assert!(new_account_frozen_status_post == new_account_frozen_status_pre);
        cvlr_assert!(old_account_frozen_status_post == old_account_frozen_status_pre);
    }
}