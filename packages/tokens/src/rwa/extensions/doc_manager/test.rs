extern crate std;

use soroban_sdk::{contract, Bytes, BytesN, Env, String, Vec};

use super::storage::{
    document_exists, get_all_documents, get_document, get_document_count, remove_document,
    set_document,
};

#[contract]
struct MockContract;

/// Helper function to create a test document hash
fn create_test_hash(e: &Env, data: &str) -> BytesN<32> {
    let bytes = Bytes::from_slice(e, data.as_bytes());
    e.crypto().sha256(&bytes).into()
}

/// Helper function to create a test document name
fn create_test_name(e: &Env, name: &str) -> BytesN<32> {
    let mut name_bytes = [0u8; 32];
    let name_slice = name.as_bytes();
    let copy_len = std::cmp::min(name_slice.len(), 32);
    name_bytes[..copy_len].copy_from_slice(&name_slice[..copy_len]);
    BytesN::from_array(e, &name_bytes)
}

#[test]
fn set_document_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "test_doc");
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "document content");

        set_document(&e, &name, &uri, &hash);

        let stored_doc = get_document(&e, &name);
        assert_eq!(stored_doc.uri, uri);
        assert_eq!(stored_doc.document_hash, hash);
        // Timestamp should be set (in test environment it may be 0)
        let _ = stored_doc.timestamp;
    });
}

#[test]
fn set_document_update_existing() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "test_doc");
        let uri1 = String::from_str(&e, "https://example.com/doc_v1.pdf");
        let hash1 = create_test_hash(&e, "document content v1");
        let uri2 = String::from_str(&e, "https://example.com/doc_v2.pdf");
        let hash2 = create_test_hash(&e, "document content v2");

        // Set initial document
        set_document(&e, &name, &uri1, &hash1);
        let doc1 = get_document(&e, &name);

        // Update the document
        set_document(&e, &name, &uri2, &hash2);
        let doc2 = get_document(&e, &name);

        // Verify update
        assert_eq!(doc2.uri, uri2);
        assert_eq!(doc2.document_hash, hash2);
        assert!(doc2.timestamp >= doc1.timestamp);

        // Verify document count didn't increase (it's an update, not a new document)
        assert_eq!(get_document_count(&e), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #380)")]
fn get_document_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "nonexistent");
        get_document(&e, &name);
    });
}

#[test]
fn remove_document_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "test_doc");
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "document content");

        // Set document
        set_document(&e, &name, &uri, &hash);
        assert!(document_exists(&e, &name));
        assert_eq!(get_document_count(&e), 1);

        // Remove document
        remove_document(&e, &name);
        assert!(!document_exists(&e, &name));
        assert_eq!(get_document_count(&e), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #380)")]
fn remove_document_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "nonexistent");
        remove_document(&e, &name);
    });
}

#[test]
fn get_all_documents_empty() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let documents = get_all_documents(&e);
        assert_eq!(documents.len(), 0);
    });
}

#[test]
fn get_all_documents_multiple() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name1 = create_test_name(&e, "doc1");
        let uri1 = String::from_str(&e, "https://example.com/doc1.pdf");
        let hash1 = create_test_hash(&e, "document 1 content");

        let name2 = create_test_name(&e, "doc2");
        let uri2 = String::from_str(&e, "https://example.com/doc2.pdf");
        let hash2 = create_test_hash(&e, "document 2 content");

        let name3 = create_test_name(&e, "doc3");
        let uri3 = String::from_str(&e, "https://example.com/doc3.pdf");
        let hash3 = create_test_hash(&e, "document 3 content");

        // Set multiple documents
        set_document(&e, &name1, &uri1, &hash1);
        set_document(&e, &name2, &uri2, &hash2);
        set_document(&e, &name3, &uri3, &hash3);

        let documents = get_all_documents(&e);
        assert_eq!(documents.len(), 3);

        // Verify all documents are present
        let mut found_names = Vec::new(&e);
        for (name, _document) in documents.iter() {
            found_names.push_back(name.clone());
        }

        assert!(found_names.contains(&name1));
        assert!(found_names.contains(&name2));
        assert!(found_names.contains(&name3));
    });
}

#[test]
fn get_all_documents_after_removal() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name1 = create_test_name(&e, "doc1");
        let uri1 = String::from_str(&e, "https://example.com/doc1.pdf");
        let hash1 = create_test_hash(&e, "document 1 content");

        let name2 = create_test_name(&e, "doc2");
        let uri2 = String::from_str(&e, "https://example.com/doc2.pdf");
        let hash2 = create_test_hash(&e, "document 2 content");

        // Set two documents
        set_document(&e, &name1, &uri1, &hash1);
        set_document(&e, &name2, &uri2, &hash2);
        assert_eq!(get_document_count(&e), 2);

        // Remove one document
        remove_document(&e, &name1);

        let documents = get_all_documents(&e);
        assert_eq!(documents.len(), 1);
        let (doc_name, _doc) = documents.get(0).unwrap();
        assert_eq!(doc_name, name2);
        assert_eq!(get_document_count(&e), 1);
    });
}

#[test]
fn document_exists_functionality() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "test_doc");
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "document content");

        // Initially doesn't exist
        assert!(!document_exists(&e, &name));

        // Set document
        set_document(&e, &name, &uri, &hash);
        assert!(document_exists(&e, &name));

        // Remove document
        remove_document(&e, &name);
        assert!(!document_exists(&e, &name));
    });
}

#[test]
fn document_count_tracking() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        assert_eq!(get_document_count(&e), 0);

        let name1 = create_test_name(&e, "doc1");
        let uri1 = String::from_str(&e, "https://example.com/doc1.pdf");
        let hash1 = create_test_hash(&e, "document 1 content");

        let name2 = create_test_name(&e, "doc2");
        let uri2 = String::from_str(&e, "https://example.com/doc2.pdf");
        let hash2 = create_test_hash(&e, "document 2 content");

        // Add first document
        set_document(&e, &name1, &uri1, &hash1);
        assert_eq!(get_document_count(&e), 1);

        // Add second document
        set_document(&e, &name2, &uri2, &hash2);
        assert_eq!(get_document_count(&e), 2);

        // Update first document (count should remain the same)
        let new_uri1 = String::from_str(&e, "https://example.com/doc1_updated.pdf");
        set_document(&e, &name1, &new_uri1, &hash1);
        assert_eq!(get_document_count(&e), 2);

        // Remove one document
        remove_document(&e, &name1);
        assert_eq!(get_document_count(&e), 1);

        // Remove last document
        remove_document(&e, &name2);
        assert_eq!(get_document_count(&e), 0);
    });
}
