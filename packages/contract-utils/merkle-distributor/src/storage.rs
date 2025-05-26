use core::marker::PhantomData;

use soroban_sdk::{contracttype, BytesN, Env, Vec};
use stellar_crypto::{hasher::Hasher, merkle::Verifier};

/// Storage keys for the data associated with `MerkleDistributor`
#[contracttype]
pub enum MerkleDistributorStorageKey {
    Root,
    Claimed(BytesN<32>),
}

pub struct MerkleDistributor<H: Hasher>(PhantomData<H>);

impl<H> MerkleDistributor<H>
where
    H: Hasher<Output = BytesN<32>>,
{
    pub fn get_root(e: &Env) -> H::Output {
        let root: BytesN<32> =
            e.storage().instance().get(&MerkleDistributorStorageKey::Root).unwrap();
        root
    }

    pub fn is_claimed(e: &Env, leaf: H::Output) -> bool {
        let key = MerkleDistributorStorageKey::Claimed(leaf);
        // TODO: panic is no key and extend
        e.storage().persistent().get(&key).unwrap_or_default()
    }

    pub fn set_root(e: &Env, root: H::Output) {
        e.storage().instance().set(&MerkleDistributorStorageKey::Root, &root);
    }

    pub fn set_claimed(e: &Env, leaf: H::Output) {
        let key = MerkleDistributorStorageKey::Claimed(leaf);
        // TODO: panic if no key and extend
        e.storage().persistent().set(&key, &true)
    }

    pub fn verify_and_set_claimed(e: &Env, leaf: H::Output, proof: Vec<H::Output>) {
        // TODO is_claimed
        let root = Self::get_root(e);

        match Verifier::<H>::verify(e, proof, root, leaf.clone()) {
            true => Self::set_claimed(e, leaf),
            false => (), // panic
        };
    }
}
