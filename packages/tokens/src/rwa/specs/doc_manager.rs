use soroban_sdk::{Address, BytesN, Env};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use cvlr_soroban_derive::rule;
use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use crate::rwa::specs::helpers::nondet;
use crate::rwa::extensions::doc_manager::{
    DocumentManager, set_document, get_document, remove_document, get_documents, 
    DocumentStorageKey, get_document_count, BUCKET_SIZE,
};

// todo invariants - swap and pop mechanism 

// helpers 
pub fn get_index_non_pancicking(e: Env, name: &BytesN<32>) -> Option<u32> {
    let key = DocumentStorageKey::Index(name.clone());
    let index = e.storage().persistent().get(&key);
    index
}

pub fn get_bucket_from_index(index: u32) -> u32 {
    index / BUCKET_SIZE
}

pub fn get_offset_from_index(index: u32) -> u32 {
    index % BUCKET_SIZE
}

// rules

#[rule]
// after set_document get Index(name) does not panic
// status: verified
pub fn set_document_integrity_1(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let uri: soroban_sdk::String = nondet_string();
    let hash = nondet_bytes_n();
    set_document(&e, &name, &uri, &hash);
    let index = get_index_non_pancicking(e, &name);
    cvlr_assert!(index.is_some());
}

#[rule]
// after remove_document get_document_count decreases by 1
// status: verified
pub fn remove_document_integrity_3(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let count_pre = get_document_count(&e);
    remove_document(&e, &name);
    let count_post = get_document_count(&e);
    cvlr_assert!(count_post == count_pre - 1);
}