/// Document management storage implementation for RWA tokens.
///
/// This module provides the core storage functionality for managing documents
/// attached to smart contracts, following the ERC-1643 standard adapted for
/// Soroban.
///
/// ## Document Storage Model
///
/// Documents are stored with the following information:
/// - **Name**: A 32-byte unique identifier for the document
/// - **URI**: A string pointing to where the document can be accessed
/// - **Hash**: A 32-byte hash of the document contents for integrity
///   verification
/// - **Timestamp**: When the document was last modified
///
/// ## Bucket System
///
/// Documents are stored in buckets of 50 entries each. Each bucket is a
/// `Vec<(BytesN<32>, Document)>` stored under its bucket index. This eliminates
/// the need for individual document storage entries and significantly reduces
/// storage costs.
///
/// ## Swap-and-Pop Pattern
///
/// When removing a document, the last document in the list is moved to fill
/// the gap left by the removed document. This keeps storage compact and
/// ensures O(1) removal operations.
use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, String, TryFromVal, Val, Vec};

use super::{
    DocumentError, BUCKET_SIZE,
    DOCUMENT_EXTEND_AMOUNT, DOCUMENT_TTL_THRESHOLD, MAX_DOCUMENTS, MAX_URI_LEN,
};

#[cfg(not(feature = "certora"))]
use super::{
    emit_document_removed, emit_document_updated,
};

/// Represents a document with its metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    /// The URI where the document can be accessed.
    pub uri: String,
    /// The hash of the document contents.
    pub document_hash: BytesN<32>,
    /// Timestamp when the document was last modified.
    pub timestamp: u64,
}

/// Storage keys for document management.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentStorageKey {
    /// Maps document name to its global index.
    Index(BytesN<32>),
    /// Maps bucket index to a vector of (name, document) tuples.
    Bucket(u32),
    /// Total count of documents.
    Count,
}

// ################## QUERY STATE ##################

/// Gets the total number of documents stored.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn get_document_count(e: &Env) -> u32 {
    get_persistent_entry(e, &DocumentStorageKey::Count).unwrap_or(0)
}

/// Retrieves the details of a document with a known name.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name (32-byte identifier).
///
/// # Errors
///
/// * [`DocumentError::DocumentNotFound`] - If no document exists with the given
///   name
pub fn get_document(e: &Env, name: &BytesN<32>) -> Document {
    let index: u32 = get_persistent_entry(e, &DocumentStorageKey::Index(name.clone()))
        .unwrap_or_else(|| panic_with_error!(e, DocumentError::DocumentNotFound));

    let (_, document) = get_document_by_index(e, index);
    document
}

/// Returns a document by its global index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `index` - The global index of the document to retrieve.
///
/// # Errors
///
/// * [`DocumentError::DocumentNotFound`] - If `index` is out of bounds.
pub fn get_document_by_index(e: &Env, index: u32) -> (BytesN<32>, Document) {
    let count = get_document_count(e);
    if index >= count {
        panic_with_error!(e, DocumentError::DocumentNotFound)
    }

    let bucket_index = index / BUCKET_SIZE;
    let offset_in_bucket = index % BUCKET_SIZE;

    let bucket: Vec<(BytesN<32>, Document)> =
        get_persistent_entry(e, &DocumentStorageKey::Bucket(bucket_index))
            .expect("bucket to be present");

    bucket.get(offset_in_bucket).expect("document entry to be present in bucket")
}

/// Retrieves documents from a specific bucket.
///
/// Returns an empty vector if the bucket is empty or doesn't exist.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `bucket_index` - The index of the bucket to retrieve documents from.
pub fn get_documents(e: &Env, bucket_index: u32) -> Vec<(BytesN<32>, Document)> {
    let bucket_key = DocumentStorageKey::Bucket(bucket_index);
    get_persistent_entry(e, &bucket_key).unwrap_or_else(|| Vec::new(e))
}

// ################## UPDATE STATE ##################

/// Attaches a new document to the contract or updates an existing one.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name (32-byte identifier).
/// * `uri` - The URI where the document can be accessed.
/// * `document_hash` - The hash of the document contents.
///
/// # Errors
///
/// * [`DocumentError::MaxDocumentsReached`] - If the maximum number of
///   documents has been reached.
/// * [`DocumentError::UriTooLong`] - If the URI exceeds the maximum allowed
///   length of 200 characters.
///
/// # Events
///
/// * topics - `["document_updated", name: BytesN<32>]`
/// * data - `[uri: String, document_hash: BytesN<32>, timestamp: u64]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In functions that implement their own authorization logic
pub fn set_document(e: &Env, name: &BytesN<32>, uri: &String, document_hash: &BytesN<32>) {
    // Validate URI length
    if uri.len() > MAX_URI_LEN {
        panic_with_error!(e, DocumentError::UriTooLong)
    }

    let timestamp = e.ledger().timestamp();

    let document = Document { uri: uri.clone(), document_hash: document_hash.clone(), timestamp };

    // Check if this is a new document or an update
    let index_key = DocumentStorageKey::Index(name.clone());
    let existing_index: Option<u32> = e.storage().persistent().get(&index_key);

    if let Some(index) = existing_index {
        // Extend TTL
        e.storage().persistent().extend_ttl(
            &index_key,
            DOCUMENT_TTL_THRESHOLD,
            DOCUMENT_EXTEND_AMOUNT,
        );
        // Update existing document in its bucket
        let bucket_index = index / BUCKET_SIZE;
        let offset_in_bucket = index % BUCKET_SIZE;
        let bucket_key = DocumentStorageKey::Bucket(bucket_index);
        let mut bucket: Vec<(BytesN<32>, Document)> =
            e.storage().persistent().get(&bucket_key).expect("bucket to be present");

        bucket.set(offset_in_bucket, (name.clone(), document.clone()));
        e.storage().persistent().set(&bucket_key, &bucket);
    } else {
        // Add new document
        let count = get_document_count(e);
        if count >= MAX_DOCUMENTS {
            panic_with_error!(e, DocumentError::MaxDocumentsReached)
        }

        e.storage().persistent().set(&index_key, &count);

        let bucket_index = count / BUCKET_SIZE;
        let bucket_key = DocumentStorageKey::Bucket(bucket_index);
        let mut bucket: Vec<(BytesN<32>, Document)> =
            e.storage().persistent().get(&bucket_key).unwrap_or_else(|| Vec::new(e));

        bucket.push_back((name.clone(), document.clone()));
        e.storage().persistent().set(&bucket_key, &bucket);

        e.storage().persistent().set(&DocumentStorageKey::Count, &(count + 1));
    }
    #[cfg(not(feature = "certora"))]
    emit_document_updated(e, name, uri, document_hash, timestamp);
}

/// Removes an existing document from the contract.
///
/// Uses a swap-remove pattern: the last document in the list is moved to fill
/// the gap left by the removed document. This keeps storage compact and
/// ensures O(1) removal operations, but means document indices can change.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name to remove.
///
/// # Events
///
/// * topics - `["document_removed", name: BytesN<32>]`
/// * data - `[]`
///
/// # Errors
///
/// * [`DocumentError::DocumentNotFound`] - If no document exists with the given
///   name
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In functions that implement their own authorization logic
pub fn remove_document(e: &Env, name: &BytesN<32>) {
    // Get the index of the document to remove
    let index_key = DocumentStorageKey::Index(name.clone());
    let document_index: u32 = e
        .storage()
        .persistent()
        .get(&index_key)
        .unwrap_or_else(|| panic_with_error!(e, DocumentError::DocumentNotFound));

    let count = get_document_count(e);
    let last_index = count - 1;

    // Get bucket information for the document to remove
    let doc_bucket_index = document_index / BUCKET_SIZE;
    let doc_offset = document_index % BUCKET_SIZE;

    // Get bucket information for the last document
    let last_bucket_index = last_index / BUCKET_SIZE;
    let last_offset = last_index % BUCKET_SIZE;

    // If this is not the last document, swap it with the last one
    if document_index != last_index {
        // Get the last document entry from its bucket
        let last_bucket_key = DocumentStorageKey::Bucket(last_bucket_index);
        let last_bucket: Vec<(BytesN<32>, Document)> =
            e.storage().persistent().get(&last_bucket_key).expect("last bucket to be present");
        let (last_name, last_doc) =
            last_bucket.get(last_offset).expect("last document entry to be present");

        // Update the last document's index to point to the removed document's position
        let last_index_key = DocumentStorageKey::Index(last_name.clone());
        e.storage().persistent().set(&last_index_key, &document_index);

        // Move the last document entry to the removed document's position
        let doc_bucket_key = DocumentStorageKey::Bucket(doc_bucket_index);
        let mut doc_bucket: Vec<(BytesN<32>, Document)> =
            e.storage().persistent().get(&doc_bucket_key).expect("document bucket to be present");
        doc_bucket.set(doc_offset, (last_name, last_doc));
        e.storage().persistent().set(&doc_bucket_key, &doc_bucket);
    }

    // Remove the last element from its bucket
    let last_bucket_key = DocumentStorageKey::Bucket(last_bucket_index);
    let mut last_bucket: Vec<(BytesN<32>, Document)> =
        e.storage().persistent().get(&last_bucket_key).expect("last bucket to be present");
    last_bucket.pop_back();
    e.storage().persistent().set(&last_bucket_key, &last_bucket);

    e.storage().persistent().remove(&index_key);

    e.storage().persistent().set(&DocumentStorageKey::Count, &last_index);
    #[cfg(not(feature = "certora"))]
    emit_document_removed(e, name);
}

// ################## HELPERS ##################

/// Helper function that tries to retrieve a persistent storage value and
/// extend its TTL if the entry exists.
///
/// # Arguments
///
/// * `e` - The Soroban reference.
/// * `key` - The key required to retrieve the underlying storage.
fn get_persistent_entry<T: TryFromVal<Env, Val>>(e: &Env, key: &DocumentStorageKey) -> Option<T> {
    e.storage().persistent().get::<_, T>(key).inspect(|_| {
        e.storage().persistent().extend_ttl(key, DOCUMENT_TTL_THRESHOLD, DOCUMENT_EXTEND_AMOUNT);
    })
}
