extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::contract::{AuditorRegistryContract, AuditorRegistryContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> AuditorRegistryContractClient<'a> {
    let address = e.register(AuditorRegistryContract, (admin, manager));
    AuditorRegistryContractClient::new(e, &address)
}

fn point(e: &Env, x_lo: u8, y_lo: u8) -> BytesN<64> {
    let mut buf = [0u8; 64];
    buf[31] = x_lo;
    buf[63] = y_lo;
    BytesN::from_array(e, &buf)
}

#[test]
fn register_and_get_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let p = point(&e, 1, 2);
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

    let old = point(&e, 1, 2);
    let new = point(&e, 3, 4);

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

    client.register_key(&1u32, &point(&e, 1, 2), &manager);
    client.register_key(&1u32, &point(&e, 3, 4), &manager);
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

    client.rotate_key(&7u32, &point(&e, 1, 2), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #3303)")]
fn register_identity_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_key(&1u32, &BytesN::from_array(&e, &[0u8; 64]), &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #3302)")]
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
#[should_panic]
fn register_by_non_manager_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let stranger = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_key(&1u32, &point(&e, 1, 2), &stranger);
}

#[test]
fn register_n_keys_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // Register a handful of distinct keys; every get_key must return its
    // registered point.
    for i in 0..10u32 {
        let p = point(&e, (i + 1) as u8, (i + 100) as u8);
        client.register_key(&i, &p, &manager);
        assert_eq!(client.get_key(&i), p);
    }

    // Verify all of them still resolve after the batch of registrations.
    for i in 0..10u32 {
        let expected = point(&e, (i + 1) as u8, (i + 100) as u8);
        assert_eq!(client.get_key(&i), expected);
    }
}
