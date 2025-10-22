extern crate std;

use soroban_sdk::{contract, Bytes, BytesN, Env, String, Vec};

use crate::rwa::extensions::doc_manager::{
    storage::{
        get_document, get_document_by_index, get_document_count, get_documents, remove_document,
        set_document,
    },
    DocumentStorageKey, BUCKET_SIZE, MAX_DOCUMENTS, MAX_URI_LEN,
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

fn document_exists(e: &Env, name: &BytesN<32>) -> bool {
    let key = DocumentStorageKey::Index(name.clone());
    e.storage().persistent().has(&key)
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
        assert_eq!(get_document_count(&e), 1);

        // Remove document
        remove_document(&e, &name);
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
fn get_documents_empty() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        // Should return empty vector when bucket is empty
        let documents = get_documents(&e, 0);
        assert_eq!(documents.len(), 0);
    });
}

#[test]
fn get_documents_multiple() {
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

        // Get documents from bucket 0
        let documents = get_documents(&e, 0);
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
fn get_documents_after_removal() {
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

        // Get documents from bucket 0
        let documents = get_documents(&e, 0);
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

#[test]
fn swap_and_pop_behavior() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        // Add 5 documents
        let name1 = create_test_name(&e, "doc1");
        let name2 = create_test_name(&e, "doc2");
        let name3 = create_test_name(&e, "doc3");
        let name4 = create_test_name(&e, "doc4");
        let name5 = create_test_name(&e, "doc5");

        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "content");

        set_document(&e, &name1, &uri, &hash);
        set_document(&e, &name2, &uri, &hash);
        set_document(&e, &name3, &uri, &hash);
        set_document(&e, &name4, &uri, &hash);
        set_document(&e, &name5, &uri, &hash);

        assert_eq!(get_document_count(&e), 5);

        // Remove doc2 (index 1) - should be replaced by doc5 (last element)
        remove_document(&e, &name2);
        assert_eq!(get_document_count(&e), 4);

        // Verify doc2 is gone
        assert!(!document_exists(&e, &name2));

        // Verify doc5 is now at index 1 (where doc2 was)
        let (doc_at_index_1, _) = get_document_by_index(&e, 1);
        assert_eq!(doc_at_index_1, name5);

        // Verify all other documents still exist
        assert!(document_exists(&e, &name1));
        assert!(document_exists(&e, &name3));
        assert!(document_exists(&e, &name4));
        assert!(document_exists(&e, &name5));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #381)")]
fn max_documents_limit() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let uri = String::from_str(&e, "https://example.com/doc.pdf");
    let hash = create_test_hash(&e, "content");

    e.as_contract(&contract_id, || {
        e.storage().persistent().set(&DocumentStorageKey::Count, &MAX_DOCUMENTS);
        let name = create_test_name(&e, "one_too_many");
        set_document(&e, &name, &uri, &hash);
    });
}

#[test]
fn get_document_by_index_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name1 = create_test_name(&e, "doc1");
        let name2 = create_test_name(&e, "doc2");
        let name3 = create_test_name(&e, "doc3");

        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "content");

        set_document(&e, &name1, &uri, &hash);
        set_document(&e, &name2, &uri, &hash);
        set_document(&e, &name3, &uri, &hash);

        // Get documents by index
        let (doc_name_0, doc_0) = get_document_by_index(&e, 0);
        assert_eq!(doc_name_0, name1);
        assert_eq!(doc_0.uri, uri);

        let (doc_name_1, _) = get_document_by_index(&e, 1);
        assert_eq!(doc_name_1, name2);

        let (doc_name_2, _) = get_document_by_index(&e, 2);
        assert_eq!(doc_name_2, name3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #380)")]
fn get_document_by_index_out_of_bounds() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "doc1");
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "content");

        set_document(&e, &name, &uri, &hash);

        // Try to get index 1 when only index 0 exists
        get_document_by_index(&e, 1);
    });
}

#[test]
fn add_document_to_new_bucket() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "content");

        // Add documents to fill first bucket (50 docs)
        for i in 0..BUCKET_SIZE {
            let name = create_test_name(&e, &std::format!("doc_bucket0_{}", i));
            set_document(&e, &name, &uri, &hash);
        }

        assert_eq!(get_document_count(&e), BUCKET_SIZE);

        // Verify first bucket is full by checking the last document
        let (last_name_bucket0, _) = get_document_by_index(&e, BUCKET_SIZE - 1);
        let expected_last_name =
            create_test_name(&e, &std::format!("doc_bucket0_{}", BUCKET_SIZE - 1));
        assert_eq!(last_name_bucket0, expected_last_name);

        // Add one more document - should go to bucket 1
        let name_bucket1 = create_test_name(&e, "doc_bucket1_0");
        set_document(&e, &name_bucket1, &uri, &hash);

        assert_eq!(get_document_count(&e), BUCKET_SIZE + 1);

        // Verify the new document is in bucket 1 (index 50)
        let (retrieved_name, retrieved_doc) = get_document_by_index(&e, BUCKET_SIZE);
        assert_eq!(retrieved_name, name_bucket1);
        assert_eq!(retrieved_doc.uri, uri);
    });
}

#[test]
fn swap_and_pop_across_different_buckets() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "content");

        // Add 75 documents (will span 2 buckets: 50 in bucket 0, 25 in bucket 1)
        for i in 0..75 {
            let name = create_test_name(&e, &std::format!("doc_{}", i));
            set_document(&e, &name, &uri, &hash);
        }

        assert_eq!(get_document_count(&e), 75);

        // Remove a document from bucket 0 (index 10)
        // This should swap with the last document (doc_74 at index 74 in bucket 1)
        let name_to_remove = create_test_name(&e, "doc_10");
        remove_document(&e, &name_to_remove);

        assert_eq!(get_document_count(&e), 74);

        // Verify doc_10 is gone
        assert!(!document_exists(&e, &name_to_remove));

        // Verify doc_74 is now at index 10 (where doc_10 was)
        let (swapped_name, _) = get_document_by_index(&e, 10);
        assert_eq!(swapped_name, create_test_name(&e, "doc_74"));

        // Verify doc_74 still exists and can be retrieved by name
        let doc_74 = get_document(&e, &create_test_name(&e, "doc_74"));
        assert_eq!(doc_74.uri, uri);
    });
}

#[test]
fn remove_last_document_in_bucket() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let uri = String::from_str(&e, "https://example.com/doc.pdf");
        let hash = create_test_hash(&e, "content");

        // Add exactly 50 documents (fills bucket 0)
        for i in 0..BUCKET_SIZE {
            let name = create_test_name(&e, &std::format!("doc_{}", i));
            set_document(&e, &name, &uri, &hash);
        }

        assert_eq!(get_document_count(&e), BUCKET_SIZE);

        // Remove the last document (index 49)
        let last_name = create_test_name(&e, &std::format!("doc_{}", BUCKET_SIZE - 1));
        remove_document(&e, &last_name);

        assert_eq!(get_document_count(&e), BUCKET_SIZE - 1);

        // Verify it's gone
        assert!(!document_exists(&e, &last_name));

        // Verify we can still access other documents
        let (first_name, _) = get_document_by_index(&e, 0);
        assert_eq!(first_name, create_test_name(&e, "doc_0"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #382)")]
fn set_document_uri_too_long() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let name = create_test_name(&e, "test_doc");
        // Create a URI that exceeds MAX_URI_LEN (200 characters)
        let long_uri = String::from_str(&e, &"a".repeat((MAX_URI_LEN + 1) as usize));
        let hash = create_test_hash(&e, "document content");

        // This should panic with UriTooLong error (382)
        set_document(&e, &name, &long_uri, &hash);
    });
}
