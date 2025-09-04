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
//! - **Event Emission**: Emit events for document operations
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
mod test;

use soroban_sdk::{contracterror, Address, BytesN, Env, String, Symbol, Vec};
pub use storage::{
    document_exists, get_all_documents, get_document, get_document_count, remove_document,
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

    /// Retrieves a full list of all documents attached to the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    fn get_all_documents(e: &Env) -> Vec<(BytesN<32>, Document)>;
}

// ################## ERRORS ##################

/// Error codes for document management operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DocumentError {
    /// The specified document was not found.
    DocumentNotFound = 380,
}

// ################## EVENTS ##################

/// Emits an event when a document is updated (added or modified).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name.
/// * `uri` - The document URI.
/// * `document_hash` - The document hash.
/// * `timestamp` - The timestamp of the operation.
pub fn emit_document_updated(
    e: &Env,
    name: &BytesN<32>,
    uri: &String,
    document_hash: &BytesN<32>,
    timestamp: u64,
) {
    let topics = (Symbol::new(e, "document_updated"), name.clone());
    e.events().publish(topics, (uri.clone(), document_hash.clone(), timestamp));
}

/// Emits an event when a document is removed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name.
pub fn emit_document_removed(e: &Env, name: &BytesN<32>) {
    let topics = (Symbol::new(e, "document_removed"), name.clone());
    e.events().publish(topics, ());
}
