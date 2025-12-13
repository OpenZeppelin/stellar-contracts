use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    consecutive::storage::NFTConsecutiveStorageKey, extensions::consecutive::Consecutive, overrides::ContractOverrides, sequential, specs::helper::is_approved_for_token
};

// ################## INTEGRITY RULES ##################

// same integrity rules from non_fungible_integrity.rs
// but the underlying functions are different.
// the code is very challenging for the prover
// need to think how we can simplify
// perhaps we can only analyze the internal functions such as update.

#[rule]
// updates balances correctly
// status: verified https://prover.certora.com/output/33158/cbbe33a98d264b0fbd4ac7ec19cffd9b
pub fn nft_consecutive_transfer_integrity_1(e: Env) {
    let to = nondet_address();
    let from = nondet_address();
    let token_id = u32::nondet();

    let balance_from_pre = Consecutive::balance(&e, &from);
    Consecutive::transfer(&e, &from, &to, token_id);
    let balance_from_post = Consecutive::balance(&e, &from);

    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - 1);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// updates balances correctly
// status: verified https://prover.certora.com/output/33158/cbbe33a98d264b0fbd4ac7ec19cffd9b
pub fn nft_consecutive_transfer_integrity_2(e: Env) {
    let to = nondet_address();
    let from = nondet_address();
    let token_id = u32::nondet();

    let balance_to_pre = Consecutive::balance(&e, &to);
    Consecutive::transfer(&e, &from, &to, token_id);
    let balance_to_post = Consecutive::balance(&e, &to);

    if to != from {
        cvlr_assert!(balance_to_post == balance_to_pre + 1);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// after transfer the token owner is set to the to address
// status: https://prover.certora.com/output/33158/08b3797b32494c0291626077382c405d
// RAZ: pretty sure we need to revert it back! i think its pointless like this.
// Note: previously this was doing `let owner_post = Consecutive::owner_of(&e, token_id);`
// which may not be necessary for finding the owner based on the change to owner in `update`.
pub fn nft_consecutive_transfer_integrity_3(e: Env) {
    let to = nondet_address();
    let from = nondet_address();
    let token_id = u32::nondet();

    Consecutive::transfer(&e, &from, &to, token_id);
    let owner_post: Option<Address> = e.storage().persistent().get(&NFTConsecutiveStorageKey::Owner(token_id));
    cvlr_assert!(owner_post.is_some() && owner_post.unwrap() == to);
}


#[rule]
// updates balances correctly
// status: verified https://prover.certora.com/output/33158/176c79a419624672a0fadb5d4023106a
// sanity: https://prover.certora.com/output/33158/733987d707ac43fea16f3d8b4f0f972c
pub fn nft_consecutive_transfer_from_integrity_1(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let token_id = u32::nondet();

    let balance_from_pre = Consecutive::balance(&e, &from);

    Consecutive::transfer_from(&e, &spender, &from, &to, token_id);

    let balance_from_post = Consecutive::balance(&e, &from);

    if to != from {
        cvlr_assert!(balance_from_post == balance_from_pre - 1);
    } else {
        cvlr_assert!(balance_from_post == balance_from_pre);
    }
}

#[rule]
// updates balances correctly
// status: verified https://prover.certora.com/output/33158/fecc4e01acdc4ef69510a81fa7f9d748
// sanity: https://prover.certora.com/output/33158/cc5dabd82a5a44e08463900b8a83bf09
pub fn nft_consecutive_transfer_from_integrity_2(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let token_id = u32::nondet();

    let balance_to_pre = Consecutive::balance(&e, &to);

    Consecutive::transfer_from(&e, &spender, &from, &to, token_id);

    let balance_to_post = Consecutive::balance(&e, &to);

    if to != from {
        cvlr_assert!(balance_to_post == balance_to_pre + 1);
    } else {
        cvlr_assert!(balance_to_post == balance_to_pre);
    }
}

#[rule]
// removes approval
// status: verified https://prover.certora.com/output/33158/0cc2e71922e94a90987fa09bd5afa9b0
// sanity: https://prover.certora.com/output/33158/f8c67f75d3f44d199703beea9db8bbff
pub fn nft_consecutive_transfer_from_integrity_3(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let token_id = u32::nondet();

    let balance_to_pre = Consecutive::balance(&e, &to);

    Consecutive::transfer_from(&e, &spender, &from, &to, token_id);

    let approval_post = Consecutive::get_approved(&e, token_id);
    cvlr_assert!(approval_post.is_none());
}

#[rule]
// after transfer_from the token owner is to
// status: verified https://prover.certora.com/output/33158/be586547df9a40fa92ee93dd56dee3ea
// sanity: https://prover.certora.com/output/33158/186bf39e78554aa1bea92331cc3bf1fc
// Note: previously this was doing `let owner_post = Consecutive::owner_of(&e, token_id);`
// which may not be necessary for finding the owner based on the change to owner in `update`.
// WIP - review
pub fn nft_consecutive_transfer_from_integrity_4(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let token_id = u32::nondet();

    Consecutive::transfer_from(&e, &spender, &from, &to, token_id);

    let owner_post: Option<Address> = e.storage().persistent().get(&NFTConsecutiveStorageKey::Owner(token_id));
    cvlr_assert!(owner_post.is_some() && owner_post.unwrap() == to);
}

#[rule]
// after approve the token owner is approved
// status: verified
// note: ±30 min and sanity unclear.
pub fn nft_consecutive_approve_integrity(e: Env) {
    let approver = nondet_address();
    clog!(cvlr_soroban::Addr(&approver));
    let approved = nondet_address();
    clog!(cvlr_soroban::Addr(&approved));
    let token_id = u32::nondet();
    clog!(token_id);
    let owner_pre = Consecutive::owner_of(&e, token_id);
    clog!(cvlr_soroban::Addr(&owner_pre));
    let live_until_ledger = u32::nondet();
    clog!(live_until_ledger);
    cvlr_assume!(live_until_ledger > 0);
    Consecutive::approve(&e, &approver, &approved, token_id, live_until_ledger);
    let is_approved_for_token_post = is_approved_for_token(&e, &approver, &approved, token_id);
    clog!(is_approved_for_token_post);
    cvlr_assert!(is_approved_for_token_post);
}

// there is no approve_for_all function

#[rule]
// batch_mint changes balance correctly
// the owner_of the first_token id is "to"
// status: violation - unclear
pub fn nft_batch_mint_integrity(e: Env) {
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount = u32::nondet();
    clog!(amount);
    let balance_pre = Consecutive::balance(&e, &to);
    clog!(balance_pre);
    let current_token_id = sequential::next_token_id(&e);
    clog!(current_token_id);
    Consecutive::batch_mint(&e, &to, amount);
    let balance_post = Consecutive::balance(&e, &to);
    clog!(balance_post);
    cvlr_assert!(balance_post == balance_pre + amount);
    let owner_of_current_token_id = Consecutive::owner_of(&e, current_token_id);
    clog!(cvlr_soroban::Addr(&owner_of_current_token_id));
    cvlr_assert!(owner_of_current_token_id == to);
}
