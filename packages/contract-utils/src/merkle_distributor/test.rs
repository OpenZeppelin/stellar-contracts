#![cfg(test)]

extern crate std;

use hex_literal::hex;
use soroban_sdk::{
    contract, contracttype, testutils::Address as _, vec, Address, BytesN, Env, Vec,
};
use stellar_event_assertion::EventAssertion;

use crate::{
    crypto::sha256::Sha256,
    merkle_distributor::{IndexableLeaf, MerkleDistributor},
};

type Bytes32 = BytesN<32>;
type Distributor = MerkleDistributor<Sha256>;

#[contract]
struct MockContract;

#[contracttype]
#[derive(Debug, Clone)]
struct LeafData {
    pub index: u32,
    pub address: Address,
    pub amount: i128,
}

impl IndexableLeaf for LeafData {
    fn index(&self) -> u32 {
        self.index
    }
}

fn get_valid_args_sorted(e: &Env) -> (Bytes32, LeafData, Vec<Bytes32>) {
    let root = Bytes32::from_array(
        e,
        &hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
    );
    let receiver = Address::from_str(e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let data = LeafData { index: 3, address: receiver, amount: 100 };
    let proof = vec![
        e,
        Bytes32::from_array(
            e,
            &hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        Bytes32::from_array(
            e,
            &hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];
    (root, data, proof)
}

fn get_valid_args_unsorted(e: &Env) -> (Bytes32, LeafData, Vec<Bytes32>) {
    // constructed with the following data and sha256 hasher
    //[
    //{ index: 0, address:
    //{ "GDD4CDYMGZ2TST2P3FI453VPP5HYKQJZXPDUQEV23V4E2BFI2U7W6BH6", amount: 101 }
    //{ index: 1, address:
    //{ "GBJWGSVNARYGOM3HSXRCTM2P7Z7D36KCH63WSE4IICTUGOFG736AR6WU", amount: 102 }
    //{ index: 2, address:
    //{ "GA5WRXT22B7I2GS4HCUXGYZWRINJYPJMIVMXGXDSBQCMRJ2EXRKSB35B", amount: 103 }
    //{ index: 3, address:
    //{ "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX", amount: 100 }
    //]
    let root = Bytes32::from_array(
        e,
        &hex!("103d1530956325ffc0b01abc52b7b3bedecf9a0b013553e864e3cecddf3bea2d"),
    );
    let receiver = Address::from_str(e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let data = LeafData { index: 3, address: receiver, amount: 100 };
    let proof = vec![
        e,
        Bytes32::from_array(
            e,
            &hex!("e79fbb8d8aa4d90b3d3c3219eefecab0e10b5c0b5624959b9e09f0192279bea3"),
        ),
        Bytes32::from_array(
            e,
            &hex!("1398dc9c81f22a62adc7956b0ad7ca1240f06ee3053e18e2a5ab4ba2184f53ce"),
        ),
    ];
    (root, data, proof)
}

#[test]
fn test_set_root_and_successful_claim_emits_events() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let root = Bytes32::from_array(&e, &[8u8; 32]);

    e.as_contract(&address, || {
        // Set root and verify event
        Distributor::set_root(&e, root.clone());
        assert_eq!(Distributor::get_root(&e), root);

        let assert = EventAssertion::new(&e, address.clone());
        assert.assert_event_count(1);

        // Set claimed and verify event
        Distributor::set_claimed(&e, 1);
        let assert = EventAssertion::new(&e, address.clone());
        assert.assert_event_count(2);
    });
}

#[test]
fn test_valid_merkle_proof_succeeds() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (root, data, proof) = get_valid_args_sorted(&e);

    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_and_set_claimed(&e, data, proof);
        assert!(Distributor::is_claimed(&e, 3))
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1300)")]
fn test_root_not_set_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let data = LeafData { index: 1, address: Address::generate(&e), amount: 100 };
    let proof = vec![&e];
    e.as_contract(&address, || Distributor::verify_and_set_claimed(&e, data, proof));
}

#[test]
#[should_panic(expected = "Error(Contract, #1301)")]
fn test_claim_already_claimed_index_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (root, data, proof) = get_valid_args_sorted(&e);
    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_and_set_claimed(&e, data.clone(), proof.clone());
        Distributor::verify_and_set_claimed(&e, data, proof);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1302)")]
fn test_verify_with_invalid_proof_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (root, _, proof) = get_valid_args_sorted(&e);
    let data = LeafData { index: 1, address: Address::generate(&e), amount: 100 };
    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_and_set_claimed(&e, data, proof);
    });
}

#[test]
fn test_unsorted_valid_merkle_proof_succeeds() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (root, data, proof) = get_valid_args_unsorted(&e);

    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_with_index_and_set_claimed(&e, data, proof);
        assert!(Distributor::is_claimed(&e, 3))
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1300)")]
fn test_unsorted_root_not_set_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let data = LeafData { index: 1, address: Address::generate(&e), amount: 100 };
    let proof = vec![&e];
    e.as_contract(&address, || Distributor::verify_with_index_and_set_claimed(&e, data, proof));
}

#[test]
#[should_panic(expected = "Error(Contract, #1301)")]
fn test_unsorted_claim_already_claimed_index_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (root, data, proof) = get_valid_args_unsorted(&e);
    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_with_index_and_set_claimed(&e, data.clone(), proof.clone());
        Distributor::verify_with_index_and_set_claimed(&e, data, proof);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1302)")]
fn test_unsorted_verify_with_invalid_proof_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (root, _, proof) = get_valid_args_unsorted(&e);
    let data = LeafData { index: 1, address: Address::generate(&e), amount: 100 };
    e.as_contract(&address, || {
        Distributor::set_root(&e, root);
        Distributor::verify_with_index_and_set_claimed(&e, data, proof);
    });
}
