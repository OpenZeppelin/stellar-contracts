extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::contract::{ConfidentialAuditorContract, ConfidentialAuditorContractClient};

/// Grumpkin generator `G = (1, Y)`, the canonical on-curve fixture.
const GRUMPKIN_G_BYTES: [u8; 64] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xcf, 0x13, 0x5e, 0x75, 0x06, 0xa4, 0x5d, 0x63,
    0x2d, 0x27, 0x0d, 0x45, 0xf1, 0x18, 0x12, 0x94, 0x83, 0x3f, 0xc4, 0x8d, 0x82, 0x3f, 0x27, 0x2c,
];

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> ConfidentialAuditorContractClient<'a> {
    let address = e.register(ConfidentialAuditorContract, (admin, manager));
    ConfidentialAuditorContractClient::new(e, &address)
}

fn generator(e: &Env) -> BytesN<64> {
    BytesN::from_array(e, &GRUMPKIN_G_BYTES)
}

fn two_generator(e: &Env) -> BytesN<64> {
    let g = generator(e);
    Grumpkin::add(e, &g, &g)
}

#[test]
fn register_and_get_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let p = generator(&e);
    client.register_key(&1u32, &p, &manager);

    assert_eq!(client.get_key(&1u32), p);
}

#[test]
fn rotate_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let old = generator(&e);
    let new = two_generator(&e);

    client.register_key(&1u32, &old, &manager);
    client.rotate_key(&1u32, &new, &manager);

    assert_eq!(client.get_key(&1u32), new);
}

#[test]
#[should_panic(expected = "Error(Contract, #3300)")]
fn register_duplicate_id_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_key(&1u32, &generator(&e), &manager);
    client.register_key(&1u32, &two_generator(&e), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn get_unknown_id_panics() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.get_key(&42u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn rotate_unknown_id_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.rotate_key(&7u32, &generator(&e), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #3302)")]
fn register_identity_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_key(&1u32, &BytesN::from_array(&e, &[0u8; 64]), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_off_curve_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // (1, 2) is canonical but `2² ≠ 1³ - 17 (mod r)`.
    let mut buf = [0u8; 64];
    buf[31] = 1;
    buf[63] = 2;

    client.register_key(&1u32, &BytesN::from_array(&e, &buf), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_non_canonical_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let mut buf = [0u8; 64];
    // Set y = 0xFF...FF, which exceeds the Grumpkin coordinate modulus.
    for byte in &mut buf[32..] {
        *byte = 0xff;
    }
    // Ensure x is non-zero so we hit the non-canonical branch, not identity.
    buf[31] = 1;

    client.register_key(&1u32, &BytesN::from_array(&e, &buf), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn register_by_non_manager_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let stranger = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_key(&1u32, &generator(&e), &stranger);
}

#[test]
fn register_n_keys_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // Register 10 distinct on-curve points by repeated addition of G; every
    // get_key must return the point that was registered under its id.
    let g = generator(&e);
    let mut points: std::vec::Vec<BytesN<64>> = std::vec::Vec::new();
    let mut p = g.clone();
    for _ in 0..10u32 {
        points.push(p.clone());
        p = Grumpkin::add(&e, &p, &g);
    }

    for (i, point) in points.iter().enumerate() {
        client.register_key(&(i as u32), point, &manager);
        assert_eq!(client.get_key(&(i as u32)), *point);
    }

    for (i, point) in points.iter().enumerate() {
        assert_eq!(client.get_key(&(i as u32)), *point);
    }
}
