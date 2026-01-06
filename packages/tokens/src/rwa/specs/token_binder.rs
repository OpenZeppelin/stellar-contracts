use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};
use crate::rwa::utils::token_binder::{bind_token, bind_tokens, is_token_bound, linked_tokens, unbind_token};
use crate::rwa::specs::nondet::nondet_vec_address;
use crate::rwa::utils::token_binder::storage::linked_token_count;

// to do invariants with these:
// get_token_by_index
// get_token_index

// helpers

pub fn clog_tokens_vector(tokens: &Vec<Address>) {
    // important to put the clogs in optional because i don't want to prevent the case of empty vector by clogs
    let token_0 = tokens.get(0);
    if let Some(token_0) = token_0 {
        clog!(cvlr_soroban::Addr(&token_0));
    }
    let token_1 = tokens.get(1);
    if let Some(token_1) = token_1 {
        clog!(cvlr_soroban::Addr(&token_1));
    }
    let length = tokens.len();
    clog!(length);
}

#[rule]
// after bind_token the token is bound
// status: verified
pub fn bind_token_integrity_1(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    bind_token(&e, &token);
    let is_token_bound = is_token_bound(&e, &token);
    clog!(is_token_bound);
    cvlr_assert!(is_token_bound);
}

#[rule]
// sanity
pub fn bind_token_integrity_1_sanity(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    bind_token(&e, &token);
    let is_token_bound = is_token_bound(&e, &token);
    clog!(is_token_bound);
    cvlr_satisfy!(true);
}

#[rule]
// after bind_token the token count is incremented
// status: verified
pub fn bind_token_integrity_2(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));  
    let token_count_pre = linked_token_count(&e);
    clog!(token_count_pre);
    bind_token(&e, &token);
    let token_count_post = linked_token_count(&e);
    clog!(token_count_post);
    cvlr_assert!(token_count_post == token_count_pre + 1);
}

#[rule]
pub fn bind_token_integrity_2_sanity(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));  
    let token_count_pre = linked_token_count(&e);
    clog!(token_count_pre);
    bind_token(&e, &token);
    let token_count_post = linked_token_count(&e);
    clog!(token_count_post);
    cvlr_satisfy!(true);
}

#[rule]
// after bind_token the token is in bound_tokens
// status: violation - review
pub fn bind_token_integrity_3(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    bind_token(&e, &token);
    let bound_tokens = linked_tokens(&e);
    clog_tokens_vector(&bound_tokens);
    let token_in_bound_tokens = bound_tokens.contains(&token);
    clog!(token_in_bound_tokens);   
    cvlr_assert!(token_in_bound_tokens);
}

#[rule]
pub fn bind_token_integrity_3_sanity(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    bind_token(&e, &token);
    let bound_tokens = linked_tokens(&e);
    clog_tokens_vector(&bound_tokens);
    let token_in_bound_tokens = bound_tokens.contains(&token);
    clog!(token_in_bound_tokens);   
    cvlr_satisfy!(true);
}

#[rule]
// after bind_tokens any token is bounded
// status: verified
pub fn bind_tokens_integrity_1(e: Env) {
    let tokens: Vec<Address> = nondet_vec_address();
    clog_tokens_vector(&tokens);
    let token: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    let token_in_tokens = tokens.contains(&token);
    clog!(token_in_tokens);
    cvlr_assume!(token_in_tokens);
    bind_tokens(&e, &tokens);
    let is_token_bound = is_token_bound(&e, &token);
    clog!(is_token_bound);
    cvlr_assert!(is_token_bound);
}

#[rule]
pub fn bind_tokens_integrity_1_sanity(e: Env) {
    let tokens = nondet_vec_address();
    clog_tokens_vector(&tokens);
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    let token_in_tokens = tokens.contains(&token);
    clog!(token_in_tokens);
    cvlr_assume!(token_in_tokens);
    bind_tokens(&e, &tokens);
    let is_token_bound = is_token_bound(&e, &token);
    clog!(is_token_bound);
    cvlr_satisfy!(true);
}

#[rule]
// after bind_tokens the token count is incremented
// status: verified
pub fn bind_tokens_integrity_2(e: Env) {
    let tokens = nondet_vec_address();
    clog_tokens_vector(&tokens);
    let tokens_length = tokens.len();
    clog!(tokens_length);
    let token_count_pre = linked_token_count(&e);
    clog!(token_count_pre);
    bind_tokens(&e, &tokens);
    let token_count_post = linked_token_count(&e);
    clog!(token_count_post);
    cvlr_assert!(token_count_post == token_count_pre + tokens_length);
}

#[rule]
pub fn bind_tokens_integrity_2_sanity(e: Env) {
    let tokens = nondet_vec_address();
    clog_tokens_vector(&tokens);
    let tokens_length = tokens.len();
    clog!(tokens_length);
    let token_count_pre = linked_token_count(&e);
    clog!(token_count_pre);
    bind_tokens(&e, &tokens);
    let token_count_post = linked_token_count(&e);
    clog!(token_count_post);
    cvlr_satisfy!(true);
}

#[rule]
// after bind_tokens any token is in bound_tokens
// status: violation - review
pub fn bind_tokens_integrity_3(e: Env) {
    let tokens: Vec<Address> = nondet_vec_address();
    clog_tokens_vector(&tokens);
    let token: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    let token_in_tokens = tokens.contains(&token);
    clog!(token_in_tokens);
    cvlr_assume!(token_in_tokens);
    bind_tokens(&e, &tokens);
    let bound_tokens = linked_tokens(&e);
    clog_tokens_vector(&bound_tokens);
    let token_in_bound_tokens = bound_tokens.contains(&token);
    clog!(token_in_bound_tokens);   
    cvlr_assert!(token_in_bound_tokens);
}

#[rule]
pub fn bind_tokens_integrity_3_sanity(e: Env) {
    let tokens: Vec<Address> = nondet_vec_address();
    clog_tokens_vector(&tokens);
    let token: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    let token_in_tokens = tokens.contains(&token);
    clog!(token_in_tokens);
    cvlr_assume!(token_in_tokens);
    bind_tokens(&e, &tokens);
    let bound_tokens = linked_tokens(&e);
    clog_tokens_vector(&bound_tokens);
    let token_in_bound_tokens = bound_tokens.contains(&token);
    clog!(token_in_bound_tokens);   
    cvlr_satisfy!(true);
}

#[rule]
pub fn unbind_token_sanity(e: Env) {
    let token = nondet_address();
    unbind_token(&e, &token);
    cvlr_satisfy!(true);
}

#[rule]
// after unbind_token the token is unbound
// status: sanity failure
pub fn unbind_token_integrity_1(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    unbind_token(&e, &token);
    let is_token_bound = is_token_bound(&e, &token);
    clog!(is_token_bound);
    cvlr_assert!(!is_token_bound);
}

#[rule]
pub fn unbind_token_integrity_1_sanity(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    unbind_token(&e, &token);
    let is_token_bound = is_token_bound(&e, &token);
    clog!(is_token_bound);
    cvlr_satisfy!(true);
}

#[rule]
// after unbind_token the token count is decremented
// status: sanity failure
pub fn unbind_token_integrity_2(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    let token_count_pre = linked_token_count(&e);
    clog!(token_count_pre);
    unbind_token(&e, &token);
    let token_count_post = linked_token_count(&e);
    clog!(token_count_post);
    cvlr_assert!(token_count_post == token_count_pre - 1);
}

#[rule]
pub fn unbind_token_integrity_2_sanity(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    let token_count_pre = linked_token_count(&e);
    clog!(token_count_pre);
    unbind_token(&e, &token);
    let token_count_post = linked_token_count(&e);
    clog!(token_count_post);
    cvlr_satisfy!(true);
}

#[rule]
// after unbind_token the token is not in bound_tokens
// status: sanity failure
pub fn unbind_token_integrity_3(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    unbind_token(&e, &token);
    let bound_tokens = linked_tokens(&e);
    clog_tokens_vector(&bound_tokens);
    let token_in_bound_tokens = bound_tokens.contains(&token);
    clog!(token_in_bound_tokens);   
    cvlr_assert!(!token_in_bound_tokens);
}

#[rule]
// sanity
// status:
pub fn unbind_token_integrity_3_sanity(e: Env) {
    let token = nondet_address();
    clog!(cvlr_soroban::Addr(&token));
    unbind_token(&e, &token);
    let bound_tokens = linked_tokens(&e);
    clog_tokens_vector(&bound_tokens);
    let token_in_bound_tokens = bound_tokens.contains(&token);
    clog!(token_in_bound_tokens);   
    cvlr_satisfy!(true);
}