#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::storage::{AllowanceData, AllowanceKey, StorageKey};

#[contract]
struct MockStorageContract;

#[test]
fn test_allowance_key_struct() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    
    // Test creating AllowanceKey
    let key = AllowanceKey {
        owner: owner.clone(),
        spender: spender.clone(),
    };
    
    // Verify the struct fields
    assert_eq!(key.owner, owner);
    assert_eq!(key.spender, spender);
}

#[test]
fn test_allowance_data_struct() {
    // Test creating AllowanceData
    let data = AllowanceData {
        amount: 100,
        live_until_ledger: 1000,
    };
    
    // Verify the struct fields
    assert_eq!(data.amount, 100);
    assert_eq!(data.live_until_ledger, 1000);
}

#[test]
fn test_storage_key_enum() {
    let e = Env::default();
    let address = Address::generate(&e);
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    
    // Test TotalSupply variant
    let key1 = StorageKey::TotalSupply;
    
    // Test Balance variant
    let key2 = StorageKey::Balance(address);
    
    // Test Allowance variant
    let allowance_key = AllowanceKey {
        owner,
        spender,
    };
    let key3 = StorageKey::Allowance(allowance_key);
    
    // Simple assertions to make sure the code executes
    // We're mostly verifying these can be created without errors
    match key1 {
        StorageKey::TotalSupply => assert!(true),
        _ => panic!("Expected TotalSupply"),
    }
    
    match key2 {
        StorageKey::Balance(_) => assert!(true),
        _ => panic!("Expected Balance"),
    }
    
    match key3 {
        StorageKey::Allowance(_) => assert!(true),
        _ => panic!("Expected Allowance"),
    }
} 