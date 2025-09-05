extern crate std;

use soroban_sdk::{contract, Bytes, BytesN, Env};

use crate::verifiers::utils::{base64_url_encode, extract_from_bytes};

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

// Tests for base64_url_encode function
#[test]
fn base64_url_encode_empty_input() {
    let input = [];
    let mut output = [0u8; 0];

    base64_url_encode(&mut output, &input);
    // Empty input should produce empty output
    assert_eq!(output.len(), 0);
}

#[test]
fn base64_url_encode_single_byte() {
    let input = [0x4D]; // 'M' in ASCII
    let mut output = [0u8; 2];

    base64_url_encode(&mut output, &input);
    // 0x4D -> "TQ" in base64url (padding removed)
    assert_eq!(output, [b'T', b'Q']);
}

#[test]
fn base64_url_encode_two_bytes() {
    let input = [0x4D, 0x61]; // "Ma" in ASCII
    let mut output = [0u8; 3];

    base64_url_encode(&mut output, &input);
    // "Ma" -> "TWE" in base64url (padding removed)
    assert_eq!(output, [b'T', b'W', b'E']);
}

#[test]
fn base64_url_encode_three_bytes() {
    let input = [0x4D, 0x61, 0x6E]; // "Man" in ASCII
    let mut output = [0u8; 4];

    base64_url_encode(&mut output, &input);
    // "Man" -> "TWFu" in base64url
    assert_eq!(output, [b'T', b'W', b'F', b'u']);
}

#[test]
fn base64_url_encode_multiple_of_three() {
    let input = [0x4D, 0x61, 0x6E, 0x20, 0x69, 0x73]; // "Man is" in ASCII
    let mut output = [0u8; 8];

    base64_url_encode(&mut output, &input);
    // "Man is" -> "TWFuIGlz" in base64url
    assert_eq!(output, [b'T', b'W', b'F', b'u', b'I', b'G', b'l', b'z']);
}

#[test]
fn base64_url_encode_binary_data() {
    let input = [0x00, 0xFF, 0xAB, 0xCD];
    let mut output = [0u8; 6];

    base64_url_encode(&mut output, &input);
    // Binary data encoding
    assert_eq!(output, [b'A', b'P', b'-', b'r', b'z', b'Q']);
}

#[test]
fn base64_url_encode_uses_url_safe_alphabet() {
    // Input that produces characters that differ between standard base64 and
    // base64url
    let input = [0x3E, 0x3F]; // Should produce '+' and '/' in standard base64, but '-' and '_' in base64url
    let mut output = [0u8; 3];

    base64_url_encode(&mut output, &input);
    // Should use URL-safe characters: '-' instead of '+', '_' instead of '/'
    assert_eq!(output, [b'P', b'j', b'8']);
}

#[test]
fn base64_url_encode_32_byte_hash() {
    // Test with a 32-byte input (common for hashes)
    let input: [u8; 32] = [
        0x4b, 0xb7, 0xa8, 0xb9, 0x96, 0x09, 0xb0, 0xb8, 0xb1, 0xd5, 0x34, 0x69, 0x4b, 0xb1, 0xf3,
        0x1f, 0x12, 0x91, 0x38, 0xa2, 0xf2, 0xa1, 0x1f, 0x8e, 0x87, 0x02, 0xee, 0xdb, 0xb7, 0x92,
        0x92, 0x2e,
    ];
    let mut output = [0u8; 43]; // 32 bytes -> 43 chars in base64url (no padding)

    base64_url_encode(&mut output, &input);

    // This should match the expected challenge from the webauthn test
    let expected = b"S7eouZYJsLix1TRpS7HzHxKROKLyoR-OhwLu27eSki4";
    assert_eq!(output, *expected);
}

#[test]
fn base64_url_encode_all_alphabet_chars() {
    // Test input that will produce all possible base64url characters
    let input = [
        0x00, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8B, 0x30, 0xD3, 0x8F, 0x41, 0x14, 0x93,
        0x51, 0x55, 0x97, 0x61, 0x96, 0x9B, 0x71, 0xD7, 0x9F, 0x82, 0x18, 0xA3, 0x92, 0x59, 0xA7,
        0xA2, 0x9A, 0xAB, 0xB2, 0xDB, 0xAF, 0xC3, 0x1C, 0xB3, 0xD3, 0x5D, 0xB7, 0xE3, 0x9E, 0xBB,
        0xF3, 0xDF, 0xBF,
    ];
    let mut output = [0u8; 64];

    base64_url_encode(&mut output, &input);

    // Verify it contains URL-safe characters (no '+' or '/')
    for &byte in &output {
        assert!(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        assert_ne!(byte, b'+');
        assert_ne!(byte, b'/');
    }
}

#[test]
fn base64_url_encode_edge_case_255() {
    let input = [0xFF, 0xFF, 0xFF]; // All bits set
    let mut output = [0u8; 4];

    base64_url_encode(&mut output, &input);
    // 0xFFFFFF -> "____" in base64url (all underscores, index 63)
    assert_eq!(output, [b'_', b'_', b'_', b'_']);
}
