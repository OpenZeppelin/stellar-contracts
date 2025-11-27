use cvlr::{cvlr_assert, nondet::*, cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_vec};
use cvlr_soroban_derive::rule;

use soroban_sdk::{Env, Vec, contracttype};

use crate::{crypto::sha256::Sha256, merkle_distributor::{IndexableLeaf, MerkleDistributor}};
use crate::merkle_distributor::specs::merkle_distributor_contract::{MerkleDistributorContract, Leaf};

#[rule]
pub fn merkle_distributor_constructor_sanity(e: Env) {
	let root_hash = nondet_bytes_n();
	let owner = nondet_address();
	MerkleDistributorContract::merkle_distributor_constructor(&e, root_hash, owner);
	cvlr_satisfy!(true);
}

#[rule]
pub fn get_root_sanity(e: Env) {
	let root = MerkleDistributorContract::get_root(&e);
	cvlr_satisfy!(true);
}

#[rule]
pub fn is_claimed_sanity(e: Env) {
	let index: u32 = nondet();
	let claimed = MerkleDistributorContract::is_claimed(&e, index);
	cvlr_satisfy!(claimed);
}

#[rule]
pub fn set_claimed_sanity(e: Env) {
	let index: u32 = nondet();
	MerkleDistributorContract::set_claimed(&e, index);
	cvlr_satisfy!(true);
}

#[rule]
pub fn claim_sanity(e: Env) {
	let leaf = Leaf::nondet();
	let proof = nondet_vec();
	MerkleDistributorContract::claim(&e, leaf, proof);
	cvlr_satisfy!(true);
}