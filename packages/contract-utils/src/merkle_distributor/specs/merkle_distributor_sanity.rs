use cvlr::{cvlr_assert, nondet::*, cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_vec};
use cvlr_soroban_derive::rule;

use soroban_sdk::{Env, Vec, contracttype};

use crate::merkle_distributor::specs::merkle_distributor_contract::Leaf;
use crate::{crypto::sha256::Sha256, merkle_distributor::{IndexableLeaf, MerkleDistributor}};

type Distributor = MerkleDistributor<Sha256>;

#[rule]
pub fn get_root_sanity(e: Env) {
	let root = nondet_bytes_n();
	Distributor::set_root(&e, root);
	let _ = Distributor::get_root(&e);
	cvlr_satisfy!(true);
}

#[rule]
pub fn is_claimed_sanity(e: Env) {
	let idx: u32 = nondet();
	let _ = Distributor::is_claimed(&e, idx);
	cvlr_satisfy!(true);
}

#[rule]
pub fn set_root_sanity(e: Env) {
    let root = nondet_bytes_n();
    Distributor::set_root(&e, root);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_claimed_sanity(e: Env) {
	let idx: u32 = nondet();
	Distributor::set_claimed(&e, idx);
	cvlr_satisfy!(true);
}


#[rule]
pub fn verify_and_set_claimed_sanity(e: Env) {
	let root = nondet_bytes_n();
	Distributor::set_root(&e, root);
	let leaf: Leaf = nondet();
	let proof = nondet_vec();
    Distributor::verify_and_set_claimed(&e, leaf, proof);
    cvlr_satisfy!(true);
}

#[rule]
pub fn verify_with_index_and_set_claimed_sanity(e: Env) {
	let root = nondet_bytes_n();
	Distributor::set_root(&e, root);
	let leaf: Leaf = nondet();
	let proof = nondet_vec();
	Distributor::verify_with_index_and_set_claimed(&e, leaf, proof);
	cvlr_satisfy!(true);
}
