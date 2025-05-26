#![cfg(test)]

extern crate std;

use hex_literal::hex;
use soroban_sdk::{contract, contracttype, vec, xdr::ToXdr, Address, BytesN, Env};
use stellar_crypto::{hasher::Hasher, sha256::Sha256};

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
fn claim_valid_leaf() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let root = Bytes32::from_array(
        &e,
        &hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
    );

    let receiver =
        Address::from_str(&e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let amount = 100;
    let data = LeafData { index: 3, address: receiver.clone(), amount };
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
        Distributor::verify_and_set_claimed(&e, leaf, proof);
    });
}
