//! # Document Manager Extension
//!
//! This module provides document management capabilities for RWA tokens,
//! allowing contracts to attach, update, and retrieve documents with associated
//! metadata.
//!
//! The Document Manager extension follows the ERC-1643 standard for document
//! management in smart contracts, adapted for the Soroban environment.
//!
//! ## Features
//!
//! - **Document Storage**: Attach documents with URI, hash, and timestamp
//! - **Document Updates**: Modify existing document metadata
//! - **Document Removal**: Remove documents from the contract
//! - **Document Retrieval**: Get individual or all documents
//!
//! ## Usage
//!
//! ```rust
//! use crate::{rwa::extensions::doc_manager::DocumentManager, token::Token};
//!
//! #[contractimpl]
//! impl DocumentManager for MyTokenContract {
//!     // Implementation of document management functions
//! }
//! ```

mod storage;
#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{contracterror, Address, BytesN, Env, String, Vec};
pub use storage::{
    get_document, get_document_by_index, get_document_count, get_documents, remove_document,
    set_document, Document, DocumentStorageKey,
};

use crate::rwa::RWAToken;

/// The Document Manager trait for managing contract documents.
///
/// This trait extends the Token functionality to provide document management
/// capabilities following the ERC-1643 standard.
pub trait DocumentManager: RWAToken {
    /// Retrieves the details of a document with a known name.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `name` - The document name (32-byte identifier).
    ///
    /// # Errors
    ///
    /// * `DocumentNotFound` - If no document exists with the given name
    fn get_document(e: &Env, name: BytesN<32>) -> Document;

    /// Attaches a new document to the contract or updates an existing one.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `name` - The document name (32-byte identifier).
    /// * `uri` - The URI where the document can be accessed.
    /// * `document_hash` - The hash of the document contents.
    /// * `operator` - The address authorizing this operation.
    ///
    /// # Errors
    ///
    /// * [`DocumentError::DocumentNotFound`]- If no document exists with the
    ///   given name.
    ///
    /// # Events
    ///
    /// * topics - `["document_updated", name: BytesN<32>]`
    /// * data - `[uri: String, document_hash: BytesN<32>, timestamp: u64]`
    fn set_document(
        e: &Env,
        name: BytesN<32>,
        uri: String,
        document_hash: BytesN<32>,
        operator: Address,
    );

    /// Removes an existing document from the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `name` - The document name to remove.
    /// * `operator` - The address authorizing this operation.
    ///
    /// # Errors
    ///
    /// * [`DocumentError::DocumentNotFound`]- If no document exists with the
    ///   given name.
    ///
    /// # Events
    ///
    /// * topics - `["document_removed", name: BytesN<32>]`
    /// * data - `[]`
    fn remove_document(e: &Env, name: BytesN<32>, operator: Address);

    /// Retrieves documents from a specific bucket.
    ///
    /// Returns an empty vector if the bucket is empty or doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `bucket_index` - The index of the bucket to retrieve documents from.
    fn get_documents(e: &Env, bucket_index: u32) -> Vec<(BytesN<32>, Document)>;
}

// ################## ERRORS ##################

/// Error codes for document management operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DocumentError {
    /// The specified document was not found.
    DocumentNotFound = 380,
    /// Maximum number of documents has been reached.
    MaxDocumentsReached = 381,
    /// The URI exceeds the maximum allowed length.
    UriTooLong = 382,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const DOCUMENT_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const DOCUMENT_TTL_THRESHOLD: u32 = DOCUMENT_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Max. number of buckets
pub const MAX_BUCKETS: u32 = 100;
/// Maximum number of document entries per bucket.
pub const BUCKET_SIZE: u32 = 50;
/// Maximum number of documents that can be stored.
pub const MAX_DOCUMENTS: u32 = BUCKET_SIZE * MAX_BUCKETS; // 5_000
/// Maximum length for document URI.
pub const MAX_URI_LEN: u32 = 200;

// ################## EVENTS ##################

/// Event emitted when a document is updated (added or modified).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentUpdated {
    #[topic]
    pub name: BytesN<32>,
    pub uri: String,
    pub document_hash: BytesN<32>,
    pub timestamp: u64,
}

/// Emits an event when a document is updated (added or modified).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name.
/// * `uri` - The document URI.
/// * `document_hash` - The document hash.
/// * `timestamp` - The timestamp of the operation.
#[cfg(not(feature = "certora"))]
pub fn emit_document_updated(
    e: &Env,
    name: &BytesN<32>,
    uri: &String,
    document_hash: &BytesN<32>,
    timestamp: u64,
) {
    DocumentUpdated {
        name: name.clone(),
        uri: uri.clone(),
        document_hash: document_hash.clone(),
        timestamp,
    }
    .publish(e);
}

/// Event emitted when a document is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentRemoved {
    #[topic]
    pub name: BytesN<32>,
}

/// Emits an event when a document is removed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name.
#[cfg(not(feature = "certora"))]
pub fn emit_document_removed(e: &Env, name: &BytesN<32>) {
    DocumentRemoved { name: name.clone() }.publish(e);
}
