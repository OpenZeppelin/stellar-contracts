#![cfg(test)]

extern crate std;

use hex_literal::hex;
use soroban_sdk::{contract, contracttype, vec, xdr::ToXdr, Address, BytesN, Env};
use stellar_crypto::{hasher::Hasher, sha256::Sha256};
use stellar_event_assertion::EventAssertion;

use crate::MerkleDistributor;

type Bytes32 = BytesN<32>;
type Distributor = MerkleDistributor<Sha256>;

#[contract]
struct MockContract;

#[contracttype]
#[derive(Clone, Debug)]
struct LeafData {
    pub index: u32,
    pub address: Address,
    pub amount: i128,
}

#[test]
fn test_valid_merkle_proof_succeeds() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let root = Bytes32::from_array(
        &e,
        &hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
    );

    let receiver =
        Address::from_str(&e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let data = LeafData { index: 3, address: receiver, amount: 100 };
    let mut hasher = Sha256::new(&e);
    hasher.update(data.to_xdr(&e));
    let leaf = hasher.finalize();
    let proof = vec![
        &e,
        Bytes32::from_array(
            &e,
            &hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        Bytes32::from_array(
            &e,
            &hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];

    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_and_set_claimed(&e, leaf.clone(), proof);
        assert!(Distributor::is_claimed(&e, leaf));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1300)")]
fn test_root_not_set_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        Distributor::get_root(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1301)")]
fn test_set_root_twice_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let root = Bytes32::from_array(&e, &[1u8; 32]);

    e.as_contract(&address, || {
        Distributor::set_root(&e, root.clone());
        Distributor::set_root(&e, root);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1302)")]
fn test_claim_already_claimed_leaf_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let root = Bytes32::from_array(&e, &[2u8; 32]);
    let leaf = Bytes32::from_array(&e, &[3u8; 32]);
    let proof = vec![&e, Bytes32::from_array(&e, &[4u8; 32])];

    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::set_claimed(&e, leaf.clone());
        Distributor::verify_and_set_claimed(&e, leaf, proof);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1303)")]
fn test_verify_with_invalid_proof_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let root = Bytes32::from_array(&e, &[5u8; 32]);
    let leaf = Bytes32::from_array(&e, &[6u8; 32]);
    let invalid_proof = vec![&e, Bytes32::from_array(&e, &[7u8; 32])];

    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_and_set_claimed(&e, leaf, invalid_proof);
    });
}

#[test]
fn test_successful_claim_emits_events() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let root = Bytes32::from_array(&e, &[8u8; 32]);
    let leaf = Bytes32::from_array(&e, &[9u8; 32]);

    e.as_contract(&address, || {
        // Set root and verify event
        Distributor::set_root(&e, root);
        let assert = EventAssertion::new(&e, address.clone());
        assert.assert_event_count(1);

        // Set claimed and verify event
        Distributor::set_claimed(&e, leaf);
        let assert = EventAssertion::new(&e, address.clone());
        assert.assert_event_count(2);
    });
}
