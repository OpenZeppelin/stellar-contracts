#![cfg(test)]
extern crate std;

use soroban_sdk::{contract, Bytes, BytesN, Env};

use crate::verifiers::utils::extract_from_bytes;

#[contract]
struct MockContract;

#[test]
fn extract_from_bytes_basic_range() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4, 5, 6, 7, 8]);

        // Extract 4 bytes from index 2 to 5 (inclusive range)
        let result: Option<BytesN<4>> = extract_from_bytes(&e, &data, 2..6);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [3, 4, 5, 6]);
    });
}

#[test]
fn extract_from_bytes_inclusive_range() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[10, 20, 30, 40, 50]);

        // Extract 3 bytes using inclusive range
        let result: Option<BytesN<3>> = extract_from_bytes(&e, &data, 1..=3);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [20, 30, 40]);
    });
}

#[test]
fn extract_from_bytes_inclusive_range_out_of_bounds() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[10, 20, 30, 40]);

        // Extract 3 bytes using inclusive range
        let result: Option<BytesN<4>> = extract_from_bytes(&e, &data, 1..=4);
        assert!(result.is_none());
    });
}

#[test]
fn extract_from_bytes_full_range() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[0xAA, 0xBB, 0xCC, 0xDD]);

        // Extract all bytes using unbounded range
        let result: Option<BytesN<4>> = extract_from_bytes(&e, &data, ..);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [0xAA, 0xBB, 0xCC, 0xDD]);
    });
}

#[test]
fn extract_from_bytes_from_start() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4, 5, 6]);

        // Extract first 3 bytes
        let result: Option<BytesN<3>> = extract_from_bytes(&e, &data, ..3);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [1, 2, 3]);
    });
}

#[test]
fn extract_from_bytes_to_end() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

        // Extract from index 2 to end
        let result: Option<BytesN<3>> = extract_from_bytes(&e, &data, 2..);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [3, 4, 5]);
    });
}

#[test]
fn extract_from_bytes_single_byte() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[0xFF, 0xEE, 0xDD]);

        // Extract single byte at index 1
        let result: Option<BytesN<1>> = extract_from_bytes(&e, &data, 1..2);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [0xEE]);
    });
}

#[test]
fn extract_from_bytes_out_of_bounds() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4]);

        // Try to extract beyond data length
        let result: Option<BytesN<3>> = extract_from_bytes(&e, &data, 3..7);
        assert!(result.is_none());
    });
}

#[test]
fn extract_from_bytes_too_many_bytes() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4]);

        // Try to extract more bytes than N allows
        let result: Option<BytesN<2>> = extract_from_bytes(&e, &data, 0..4);
        assert!(result.is_none());
    });
}

#[test]
fn extract_from_bytes_empty_range() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4]);

        // Extract zero bytes (empty range)
        let result: Option<BytesN<0>> = extract_from_bytes(&e, &data, 2..2);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.len(), 0);
    });
}

#[test]
fn extract_from_bytes_exact_fit() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

        // Extract exactly N bytes
        let result: Option<BytesN<5>> = extract_from_bytes(&e, &data, ..);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert_eq!(extracted.to_array(), [1, 2, 3, 4, 5]);
    });
}

#[test]
fn extract_from_bytes_edge_cases() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let data = Bytes::from_array(&e, &[0xAB, 0xCD]);

        // Extract from end boundary
        let result: Option<BytesN<1>> = extract_from_bytes(&e, &data, 1..2);
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_array(), [0xCD]);

        // Extract from start boundary
        let result: Option<BytesN<1>> = extract_from_bytes(&e, &data, 0..1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_array(), [0xAB]);
    });
}
