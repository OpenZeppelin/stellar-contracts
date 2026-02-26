extern crate std;

use soroban_sdk::{contract, testutils::Events, BytesN, Env, Event, Vec};

use crate::dkim_registry::{
    storage::{
        is_key_hash_revoked, is_key_hash_valid, revoke_dkim_public_key_hash,
        set_dkim_public_key_hash, set_dkim_public_key_hashes,
    },
    KeyHashRegistered, KeyHashRevoked,
};

#[contract]
struct MockContract;

fn domain_hash(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[1u8; 32])
}

fn public_key_hash(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[2u8; 32])
}

fn other_public_key_hash(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[3u8; 32])
}

fn other_domain_hash(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[4u8; 32])
}

#[test]
fn set_and_query_key_hash() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh = public_key_hash(&e);

        set_dkim_public_key_hash(&e, &dh, &pkh);
        assert!(is_key_hash_valid(&e, &dh, &pkh));

        let events = e.events().all();
        assert_eq!(events.events().len(), 1);
        let event = events.events().first().unwrap();
        let expected = KeyHashRegistered { domain_hash: dh.clone(), public_key_hash: pkh.clone() }
            .to_xdr(&e, &address);
        assert_eq!(event, &expected);
    });
}

#[test]
fn query_unregistered_returns_false() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh = public_key_hash(&e);

        assert!(!is_key_hash_valid(&e, &dh, &pkh));
    });
}

#[test]
fn revoke_key_hash() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh = public_key_hash(&e);

        set_dkim_public_key_hash(&e, &dh, &pkh);
        assert!(is_key_hash_valid(&e, &dh, &pkh));

        revoke_dkim_public_key_hash(&e, &pkh);
        assert!(!is_key_hash_valid(&e, &dh, &pkh));
        assert!(is_key_hash_revoked(&e, &pkh));

        let events = e.events().all();
        assert_eq!(events.events().len(), 2);
        let revoke_event = events.events().last().unwrap();
        let expected = KeyHashRevoked { public_key_hash: pkh.clone() }.to_xdr(&e, &address);
        assert_eq!(revoke_event, &expected);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #6001)")]
fn set_already_registered_key_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh = public_key_hash(&e);

        set_dkim_public_key_hash(&e, &dh, &pkh);
        set_dkim_public_key_hash(&e, &dh, &pkh);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #6000)")]
fn set_revoked_key_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh = public_key_hash(&e);

        revoke_dkim_public_key_hash(&e, &pkh);
        set_dkim_public_key_hash(&e, &dh, &pkh);
    });
}

#[test]
fn set_multiple_key_hashes() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh1 = public_key_hash(&e);
        let pkh2 = other_public_key_hash(&e);

        let hashes = Vec::from_array(&e, [pkh1.clone(), pkh2.clone()]);
        set_dkim_public_key_hashes(&e, &dh, &hashes);

        assert!(is_key_hash_valid(&e, &dh, &pkh1));
        assert!(is_key_hash_valid(&e, &dh, &pkh2));

        let events = e.events().all();
        assert_eq!(events.events().len(), 2);

        let event1 = events.events().first().unwrap();
        let expected1 =
            KeyHashRegistered { domain_hash: dh.clone(), public_key_hash: pkh1.clone() }
                .to_xdr(&e, &address);
        assert_eq!(event1, &expected1);

        let event2 = events.events().last().unwrap();
        let expected2 =
            KeyHashRegistered { domain_hash: dh.clone(), public_key_hash: pkh2.clone() }
                .to_xdr(&e, &address);
        assert_eq!(event2, &expected2);
    });
}

#[test]
fn revoke_does_not_affect_other_keys() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh = domain_hash(&e);
        let pkh1 = public_key_hash(&e);
        let pkh2 = other_public_key_hash(&e);

        set_dkim_public_key_hash(&e, &dh, &pkh1);
        set_dkim_public_key_hash(&e, &dh, &pkh2);

        revoke_dkim_public_key_hash(&e, &pkh1);

        assert!(!is_key_hash_valid(&e, &dh, &pkh1));
        assert!(is_key_hash_valid(&e, &dh, &pkh2));
    });
}

#[test]
fn same_key_different_domains() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let dh1 = domain_hash(&e);
        let dh2 = other_domain_hash(&e);
        let pkh = public_key_hash(&e);

        set_dkim_public_key_hash(&e, &dh1, &pkh);
        set_dkim_public_key_hash(&e, &dh2, &pkh);

        assert!(is_key_hash_valid(&e, &dh1, &pkh));
        assert!(is_key_hash_valid(&e, &dh2, &pkh));
    });
}
