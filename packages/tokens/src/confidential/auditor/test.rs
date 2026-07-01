extern crate std;

use soroban_sdk::{contract, testutils::Events, BytesN, Env};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::confidential::auditor::storage::{
    get_key, register_key, rotate_key, validate_point, AuditorStorageKey,
};

#[contract]
struct MockContract;

/// Grumpkin generator `G = (1, Y)` with `Y =
/// 17631683881184975370165255887551781615748388533673675138860`.
///
/// This is the canonical on-curve point shipped with `ark-grumpkin`; we use
/// it as a deterministic test fixture so the auditor tests don't depend on
/// `ark-grumpkin` themselves.
const GRUMPKIN_G_BYTES: [u8; 64] = [
    // x = 1 (32-byte big-endian)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    // y (32-byte big-endian)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xcf, 0x13, 0x5e, 0x75, 0x06, 0xa4, 0x5d, 0x63,
    0x2d, 0x27, 0x0d, 0x45, 0xf1, 0x18, 0x12, 0x94, 0x83, 0x3f, 0xc4, 0x8d, 0x82, 0x3f, 0x27, 0x2c,
];

fn generator(e: &Env) -> BytesN<64> {
    BytesN::from_array(e, &GRUMPKIN_G_BYTES)
}

/// A second on-curve sample: `2G`, obtained by doubling the generator on the
/// real Grumpkin curve.
fn two_generator(e: &Env) -> BytesN<64> {
    let g = generator(e);
    Grumpkin::add(e, &g, &g)
}

#[test]
fn register_and_get_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let p = generator(&e);

    e.as_contract(&address, || {
        register_key(&e, 1, &p);

        assert_eq!(get_key(&e, 1), p);

        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn register_multiple_keys_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Build a sequence of distinct on-curve points by repeated addition
        // of G on the real Grumpkin curve, then verify every get_key
        // round-trips.
        let g = generator(&e);
        let mut p = g.clone();
        for i in 0..5u32 {
            register_key(&e, i, &p);
            assert_eq!(get_key(&e, i), p);
            p = Grumpkin::add(&e, &p, &g);
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
        register_key(&e, 1, &generator(&e));
        register_key(&e, 1, &two_generator(&e));
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

    e.as_contract(&address, || {
        let old = generator(&e);
        let new = two_generator(&e);

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
        rotate_key(&e, 7, &generator(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3302)")]
fn register_identity_point_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_key(&e, 1, &BytesN::from_array(&e, &[0u8; 64]));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3302)")]
fn rotate_to_identity_point_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_key(&e, 1, &generator(&e));
        rotate_key(&e, 1, &BytesN::from_array(&e, &[0u8; 64]));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_off_curve_point_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // (1, 2) is canonical but `2² ≠ 1³ - 17 (mod r)`, so it is off the
        // Grumpkin curve.
        let mut buf = [0u8; 64];
        buf[31] = 1;
        buf[63] = 2;
        register_key(&e, 1, &BytesN::from_array(&e, &buf));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_non_canonical_x_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // `x = r` is the smallest non-canonical encoding; the Grumpkin
        // validator rejects it as off-curve to keep byte equality sound.
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&[
            0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81,
            0x58, 0x5d, 0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93,
            0xf0, 0x00, 0x00, 0x01,
        ]);
        buf[63] = 1;

        register_key(&e, 1, &BytesN::from_array(&e, &buf));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_non_canonical_y_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let mut buf = [0u8; 64];
        buf[31] = 1;
        // `y = 2^256 - 1` is well above `r`.
        for byte in &mut buf[32..] {
            *byte = 0xff;
        }

        register_key(&e, 1, &BytesN::from_array(&e, &buf));
    });
}

#[test]
fn rotate_emits_rotation_event() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let old = generator(&e);
        let new = two_generator(&e);

        register_key(&e, 1, &old);
        rotate_key(&e, 1, &new);

        // 2 events: AuditorRegistered + AuditorRotated.
        assert_eq!(e.events().all().events().len(), 2);
    });
}

#[test]
fn validate_point_accepts_generator() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        validate_point(&e, &generator(&e));
    });
}

#[test]
fn storage_key_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Direct read by storage key matches the value returned by get_key.
        let p = generator(&e);
        register_key(&e, 9, &p);

        let stored: BytesN<64> = e.storage().persistent().get(&AuditorStorageKey::Key(9)).unwrap();
        assert_eq!(stored, p);
    });
}
