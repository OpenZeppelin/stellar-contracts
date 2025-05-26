use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, Vec};
use stellar_crypto::{hasher::Hasher, merkle::Verifier};

use crate::{
    merkle_distributor::{emit_set_claimed, emit_set_root, MerkleDistributorError},
    MerkleDistributor,
};

/// Storage keys for the data associated with `MerkleDistributor`
#[contracttype]
pub enum MerkleDistributorStorageKey {
    Root,
    Claimed(BytesN<32>),
}

impl<H> MerkleDistributor<H>
where
    H: Hasher<Output = BytesN<32>>,
{
    pub fn get_root(e: &Env) -> H::Output {
        if let Some(root) = e.storage().instance().get(&MerkleDistributorStorageKey::Root) {
            root
        } else {
            panic_with_error!(e, MerkleDistributorError::RootNotSet);
        }
    }

    pub fn is_claimed(e: &Env, leaf: H::Output) -> bool {
        let key = MerkleDistributorStorageKey::Claimed(leaf);
        // TODO: extend
        e.storage().persistent().get(&key).unwrap_or_default()
    }

    pub fn set_root(e: &Env, root: H::Output) {
        e.storage().instance().set(&MerkleDistributorStorageKey::Root, &root);
        emit_set_root(e, root.into());
    }

    pub fn set_claimed(e: &Env, leaf: H::Output) {
        let key = MerkleDistributorStorageKey::Claimed(leaf.clone());
        // TODO: extend
        e.storage().persistent().set(&key, &true);
        emit_set_claimed(e, leaf.into());
    }

    pub fn verify_and_set_claimed(e: &Env, leaf: H::Output, proof: Vec<H::Output>) {
        if Self::is_claimed(e, leaf.clone()) {
            panic_with_error!(e, MerkleDistributorError::LeafIsClaimed);
        }

        let root = Self::get_root(e);

        match Verifier::<H>::verify(e, proof, root, leaf.clone()) {
            true => Self::set_claimed(e, leaf),
            false => panic_with_error!(e, MerkleDistributorError::InvalidProof),
        };
    }
}
