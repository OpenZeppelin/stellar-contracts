use cvlr::{cvlr_assert, nondet::*, cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_vec};
use cvlr_soroban_derive::rule;
use cvlr::clog;

use soroban_sdk::{Env, Vec, contracttype};

use crate::{crypto::sha256::Sha256, merkle_distributor::{IndexableLeaf, MerkleDistributor}};
use crate::merkle_distributor::specs::merkle_distributor_contract::{MerkleDistributorContract, Leaf};

#[rule]
// status: violated - spurious
pub fn merkle_distributor_constructor_integrity(e: Env) {
	let root_hash = nondet_bytes_n();
	let owner = nondet_address();
	clog!(cvlr_soroban::Addr(&owner));
	MerkleDistributorContract::merkle_distributor_constructor(&e, root_hash.clone(), owner);
	let root = MerkleDistributorContract::get_root(&e);
	cvlr_assert!(root == root_hash);
}

#[rule]
// status: violated - spurious
pub fn set_claimed_integrity(e: Env) {
	let index: u32 = nondet();
	clog!(index);
	MerkleDistributorContract::set_claimed(&e, index);
	let claimed = MerkleDistributorContract::is_claimed(&e, index);
	clog!(claimed);
	cvlr_assert!(claimed);
}

#[rule]
// status: violated - spurious
pub fn claim_integrity(e: Env) {
	let leaf = Leaf::nondet();
	let proof = nondet_vec();
	MerkleDistributorContract::claim(&e, leaf.clone(), proof);
	let claimed = MerkleDistributorContract::is_claimed(&e, leaf.index);
	clog!(claimed);
	cvlr_assert!(claimed);
}