use cvlr::nondet::Nondet;
use soroban_sdk::contracttype;
use soroban_sdk::symbol_short;
use soroban_sdk::Symbol;

use crate::crypto::sha256::Sha256;
use crate::merkle_distributor::IndexableLeaf;
use crate::merkle_distributor::MerkleDistributor;
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

#[contracttype]
#[derive(Clone)]
pub(crate) struct Leaf {
    pub index: u32
}

impl Nondet for Leaf {
    fn nondet() -> Self {
        Leaf {
            index: u32::nondet(),
        }
    }
}

impl IndexableLeaf for Leaf {
    fn index(&self) -> u32 {
        self.index
    }
}   

type MerkleDistributorSha256 = MerkleDistributor<Sha256>;


pub const OWNER: Symbol = symbol_short!("OWNER");

#[contract]
pub struct MerkleDistributorContract;

#[contractimpl]
impl MerkleDistributorContract {
    // sorted merkle tree
    pub fn merkle_distributor_constructor(e: &Env, root_hash: BytesN<32>, owner: Address) {
        MerkleDistributorSha256::set_root(&e, root_hash);
        e.storage().instance().set(&OWNER, &owner);
    }

    pub fn get_root(e: &Env) -> BytesN<32> {
        MerkleDistributorSha256::get_root(e)
    }

    pub fn is_claimed(e: &Env, index: u32) -> bool {
        MerkleDistributorSha256::is_claimed(e, index)
    }

    pub fn set_claimed(e: &Env, index: u32) {
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        owner.require_auth();
        MerkleDistributorSha256::set_claimed(e, index);
    }

    pub fn claim(e: &Env, leaf: Leaf, proof: Vec<BytesN<32>>) {
        MerkleDistributorSha256::verify_and_set_claimed(e, leaf, proof);
    }
}
