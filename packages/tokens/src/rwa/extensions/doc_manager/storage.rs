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
/// ## Storage Keys
///
/// - `Document(BytesN<32>)` - Maps document name to document data
/// - `DocumentList` - Maintains a list of all document names for enumeration
use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, String, Vec};

use super::{emit_document_removed, emit_document_updated, DocumentError};

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
    /// Maps document name to document data (32-byte identifier).
    Document(BytesN<32>),
    /// List of all document names.
    DocumentList,
}

// ################## QUERY STATE ##################

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
    let key = DocumentStorageKey::Document(name.clone());
    e.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, DocumentError::DocumentNotFound))
}

/// Retrieves a full list of all documents attached to the contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn get_all_documents(e: &Env) -> Vec<(BytesN<32>, Document)> {
    let list_key = DocumentStorageKey::DocumentList;
    let document_names: Vec<BytesN<32>> =
        e.storage().persistent().get(&list_key).unwrap_or_else(|| Vec::new(e));

    let mut documents = Vec::new(e);
    for name in document_names.iter() {
        if let Some(document) =
            e.storage().persistent().get(&DocumentStorageKey::Document(name.clone()))
        {
            documents.push_back((name.clone(), document));
        }
    }
    documents
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
    let timestamp = e.ledger().timestamp();

    let document = Document { uri: uri.clone(), document_hash: document_hash.clone(), timestamp };

    // Store the document
    let doc_key = DocumentStorageKey::Document(name.clone());
    e.storage().persistent().set(&doc_key, &document);

    // Update the document list if this is a new document
    let list_key = DocumentStorageKey::DocumentList;
    let mut document_names: Vec<BytesN<32>> =
        e.storage().persistent().get(&list_key).unwrap_or_else(|| Vec::new(e));

    // Check if document name already exists in the list
    let mut found = false;
    for existing_name in document_names.iter() {
        if existing_name == *name {
            found = true;
            break;
        }
    }

    // Add to list if it's a new document
    if !found {
        document_names.push_back(name.clone());
        e.storage().persistent().set(&list_key, &document_names);
    }

    // Emit event
    emit_document_updated(e, name, uri, document_hash, timestamp);
}

/// Removes an existing document from the contract.
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
    if !document_exists(e, name) {
        panic_with_error!(e, DocumentError::DocumentNotFound)
    }

    // Remove the document
    e.storage().persistent().remove(&DocumentStorageKey::Document(name.clone()));

    // Remove from the document list
    let list_key = DocumentStorageKey::DocumentList;
    let document_names: Vec<BytesN<32>> =
        e.storage().persistent().get(&list_key).unwrap_or_else(|| Vec::new(e));

    // Find and remove the document name from the list
    let mut new_names = Vec::new(e);
    for existing_name in document_names.iter() {
        if existing_name != *name {
            new_names.push_back(existing_name.clone());
        }
    }

    e.storage().persistent().set(&list_key, &new_names);

    // Emit event
    emit_document_removed(e, name);
}

// ################## HELPER FUNCTIONS ##################

/// Checks if a document exists with the given name.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `name` - The document name to check.
pub fn document_exists(e: &Env, name: &BytesN<32>) -> bool {
    let key = DocumentStorageKey::Document(name.clone());
    e.storage().persistent().has(&key)
}

/// Gets the total number of documents stored.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn get_document_count(e: &Env) -> u32 {
    let list_key = DocumentStorageKey::DocumentList;
    let document_names: Vec<BytesN<32>> =
        e.storage().persistent().get(&list_key).unwrap_or_else(|| Vec::new(e));

    document_names.len()
}
