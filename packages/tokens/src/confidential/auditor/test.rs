extern crate std;

use soroban_sdk::{contract, BytesN, Env};
use stellar_event_assertion::EventAssertion;

use crate::confidential::auditor::storage::{
    get_key, register_key, rotate_key, validate_point, AuditorStorageKey,
};

#[contract]
struct MockContract;

/// Builds a `BytesN<64>` from a 32-byte `x` and a 32-byte `y`.
fn point(e: &Env, x: [u8; 32], y: [u8; 32]) -> BytesN<64> {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(&x);
    buf[32..].copy_from_slice(&y);
    BytesN::from_array(e, &buf)
}

/// A canonical, non-identity sample point. The stubbed on-curve check
/// accepts any such point; once the Grumpkin arithmetic crate lands the
/// on-curve sample will be generated from an actual scalar.
fn sample_point(e: &Env) -> BytesN<64> {
    let mut x = [0u8; 32];
    x[31] = 1;
    let mut y = [0u8; 32];
    y[31] = 2;
    point(e, x, y)
}

fn another_sample_point(e: &Env) -> BytesN<64> {
    let mut x = [0u8; 32];
    x[31] = 3;
    let mut y = [0u8; 32];
    y[31] = 4;
    point(e, x, y)
}

#[test]
fn register_and_get_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let p = sample_point(&e);

    e.as_contract(&address, || {
        register_key(&e, 1, &p);

        assert_eq!(get_key(&e, 1), p);

        let assertion = EventAssertion::new(&e, address.clone());
        assertion.assert_event_count(1);
    });
}

#[test]
fn register_multiple_keys_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Register a handful of keys under distinct ids and verify every
        // get_key call round-trips to the registered point.
        for i in 0..5u32 {
            let mut x = [0u8; 32];
            x[31] = (i + 1) as u8;
            let mut y = [0u8; 32];
            y[31] = (i + 10) as u8;
            let p = point(&e, x, y);
            register_key(&e, i, &p);
            assert_eq!(get_key(&e, i), p);
        }
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3300)")]
fn register_duplicate_id_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_key(&e, 1, &sample_point(&e));
        register_key(&e, 1, &another_sample_point(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn get_unknown_id_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let _ = get_key(&e, 42);
    });
}

#[test]
fn rotate_key_replaces_in_place() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let old = sample_point(&e);
    let new = another_sample_point(&e);

    e.as_contract(&address, || {
        register_key(&e, 1, &old);
        rotate_key(&e, 1, &new);

        assert_eq!(get_key(&e, 1), new);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn rotate_unknown_id_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        rotate_key(&e, 7, &sample_point(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_identity_point_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_key(&e, 1, &BytesN::from_array(&e, &[0u8; 64]));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn rotate_to_identity_point_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_key(&e, 1, &sample_point(&e));
        rotate_key(&e, 1, &BytesN::from_array(&e, &[0u8; 64]));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3302)")]
fn register_non_canonical_x_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // `x = r` is the smallest non-canonical encoding.
        let x: [u8; 32] = [
            0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81,
            0x58, 0x5d, 0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93,
            0xf0, 0x00, 0x00, 0x01,
        ];
        let mut y = [0u8; 32];
        y[31] = 1;

        register_key(&e, 1, &point(&e, x, y));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3302)")]
fn register_non_canonical_y_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let mut x = [0u8; 32];
        x[31] = 1;
        // `y = 2^256 - 1` is well above `r`.
        let y = [0xffu8; 32];

        register_key(&e, 1, &point(&e, x, y));
    });
}

#[test]
fn rotate_emits_rotation_event() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let old = sample_point(&e);
    let new = another_sample_point(&e);

    e.as_contract(&address, || {
        register_key(&e, 1, &old);
        rotate_key(&e, 1, &new);

        // 2 events: AuditorRegistered + AuditorRotated.
        EventAssertion::new(&e, address.clone()).assert_event_count(2);
    });
}

#[test]
fn validate_point_accepts_canonical_non_identity() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        validate_point(&e, &sample_point(&e));
    });
}

#[test]
fn storage_key_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Direct read by storage key matches the value returned by get_key.
        let p = sample_point(&e);
        register_key(&e, 9, &p);

        let stored: BytesN<64> = e.storage().persistent().get(&AuditorStorageKey::Key(9)).unwrap();
        assert_eq!(stored, p);
    });
}
