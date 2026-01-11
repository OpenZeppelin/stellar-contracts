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
// after set_document get_document returns a doc with the right uri
// status: violation
pub fn set_document_integrity_2(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let uri: soroban_sdk::String = nondet_string();
    let hash = nondet_bytes_n();
    set_document(&e, &name, &uri, &hash);
    let doc = get_document(&e, &name);
    cvlr_assert!(doc.uri == uri);
}

#[rule]
// after set_document get_document returns a doc with the right hash
// status: violation
pub fn set_document_integrity_3(e: Env) {
    let name = nondet_bytes_n();
    let uri = nondet_string();
    let hash = nondet_bytes_n();
    set_document(&e, &name, &uri, &hash);
    let doc = get_document(&e, &name);
    cvlr_assert!(doc.document_hash == hash);
}

#[rule]
// after set_document get_document returns a doc with the right timestamp
// status: violation
pub fn set_document_integrity_4(e: Env) {
    let name = nondet_bytes_n();
    let uri = nondet_string();
    let hash = nondet_bytes_n();
    let timestamp_now = e.ledger().timestamp();
    set_document(&e, &name, &uri, &hash);
    let doc = get_document(&e, &name);
    cvlr_assert!(doc.timestamp == timestamp_now);
}

#[rule]
// after set_document get_document_count increases by 1
// status: violation
pub fn set_document_integrity_5(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let uri: soroban_sdk::String = nondet_string();
    let hash = nondet_bytes_n();
    let count_pre = get_document_count(&e);
    set_document(&e, &name, &uri, &hash);
    let count_post = get_document_count(&e);
    cvlr_assert!(count_post == count_pre + 1);
}

#[rule]
// after set_document the docs appears in get_documents is some
// status: violation
pub fn set_document_integrity_6(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let uri: soroban_sdk::String = nondet_string();
    let hash = nondet_bytes_n();
    set_document(&e, &name, &uri, &hash);
    let index = get_index_non_pancicking(e.clone(), &name).unwrap();
    let bucket = get_bucket_from_index(index);
    let offset = get_offset_from_index(index);
    let docs = get_documents(&e, bucket);
    let docs_at_offset = docs.get(offset);
    cvlr_assert!(docs_at_offset.is_some());
}

#[rule]
// after set_document the doc that appears in get_documents has the right name
// status: violation
pub fn set_document_integrity_7(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let uri: soroban_sdk::String = nondet_string();
    let hash = nondet_bytes_n();
    set_document(&e, &name, &uri, &hash);
    let index = get_index_non_pancicking(e.clone(), &name).unwrap();
    let bucket = get_bucket_from_index(index);
    let offset = get_offset_from_index(index);
    let docs = get_documents(&e, bucket);
    let docs_at_offset = docs.get(offset);
    let doc_at_offset_name = docs_at_offset.unwrap().0;
    cvlr_assert!(doc_at_offset_name == name);
}

#[rule]
// after remove_document get_index returns none
// status: violation
pub fn remove_document_integrity_1(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    remove_document(&e, &name);
    let index = get_index_non_pancicking(e, &name);
    cvlr_assert!(index.is_none());
}

#[rule]
// after remove_document get_documents for any index cannot contain the doc
// status: violation
pub fn remove_document_integrity_2(e: Env) {
    let name: BytesN<32> = nondet_bytes_n();
    let index = nondet();
    let doc = get_document(&e, &name);
    remove_document(&e, &name);
    let docs_for_index_post = get_documents(&e, index);
    let docs_contains_name = docs_for_index_post.contains(&(name, doc));
    cvlr_assert!(!docs_contains_name);
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