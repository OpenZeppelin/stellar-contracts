//! This module deals with verification of Merkle Tree proofs. It was adapted
//! from [rust-contracts-stylus](https://github.com/OpenZeppelin/rust-contracts-stylus/blob/main/lib/crypto/src/merkle.rs) to work with Soroban contract.
//!
//! The tree and the proofs can be generated using `OpenZeppelin`'s
//! [merkle tree library](https://github.com/OpenZeppelin/merkle-tree). You will
//! find a quickstart guide in its README.
//!
//! WARNING: You should avoid using leaf values that are 64 bytes long
//! prior to hashing, or use a hash function other than keccak256 for
//! hashing leaves. This is because the concatenation of a sorted pair
//! of internal nodes in the Merkle tree could be reinterpreted as a
//! leaf value. `OpenZeppelin`'s JavaScript library generates Merkle trees
//! that are safe against this attack out of the box.
use core::marker::PhantomData;

use soroban_sdk::{BytesN, Env, Vec};

use crate::{hashable::commutative_hash_pair, hasher::Hasher};

type Bytes32 = BytesN<32>;

/// Verify merkle proofs.
pub struct Verifier<H: Hasher>(PhantomData<H>);

impl<H> Verifier<H>
where
    H: Hasher<Output = Bytes32>,
{
    /// Verify that `leaf` is part of a Merkle tree defined by `root` by using
    /// `proof` and a custom hashing algorithm defined by `builder`. See
    /// [`BuildHasher`] for more information on how to construct a builder.
    ///
    /// # Arguments
    ///
    /// * `proof` - A slice of hashes that constitute the merkle proof.
    /// * `root` - The root of the merkle tree, in bytes.
    /// * `leaf` - The leaf of the merkle tree to proof, in bytes.
    #[must_use]
    pub fn verify(e: &Env, proof: Vec<Bytes32>, root: Bytes32, mut leaf: Bytes32) -> bool {
        for hash in proof {
            leaf = commutative_hash_pair(&leaf, &hash, H::new(e));
        }

        leaf == root
    }
}

#[cfg(test)]
mod tests {
    //! NOTE: The values used as input for these tests were all generated using
    //! <https://github.com/OpenZeppelin/merkle-tree>.

    extern crate std;

    use std::format;

    use hex_literal::hex;
    use proptest::{prelude::*, prop_compose};
    use soroban_sdk::Env;

    use super::{commutative_hash_pair, Bytes32, Verifier};
    use crate::{hasher::Hasher, keccak::Keccak256};

    macro_rules! to_bytes {
        ($env:tt, $lit:literal) => {
            Bytes32::from_array(&$env, &hex!($lit))
        };
    }

    macro_rules! to_bytes_array {
        ($env:tt, $($lit:literal),*) => {{
            let mut vec = soroban_sdk::Vec::new(&$env);
            $(
                vec.push_back(to_bytes!($env, $lit));
            )*
            vec
        }};
    }

    prop_compose! {
        fn valid_merkle_proof(e: Env, min_proof_len: usize)(
            leaf: [u8; 32],
            proof in prop::collection::vec(any::<[u8; 32]>(), min_proof_len..ProptestConfig::default().max_default_size_range),
        ) -> (soroban_sdk::Vec<Bytes32>, Bytes32, Bytes32) {
            let mut current = Bytes32::from_array(&e, &leaf);
            let mut proof_vec: soroban_sdk::Vec<Bytes32> = soroban_sdk::Vec::new(&e);
            for hash in &proof {
                let hash = Bytes32::from_array(&e, &hash.clone());
                proof_vec.push_back(hash.clone());
                current = commutative_hash_pair(
                    &current,
                    &hash,
                    Keccak256::new(&e),
                );
            }
            let root = current;
            let leaf = Bytes32::from_array(&e, &leaf);
            (proof_vec, root, leaf)
        }
    }

    #[test]
    fn proof_tampering_invalidates() {
        let e = Env::default();
        // Turn off the CPU/memory budget for testing.
        e.cost_estimate().budget().reset_unlimited();

        proptest!(
            |((proof, root, leaf) in valid_merkle_proof(e.clone(), 0),
             tamper_idx in 0..32usize)| {
                if let Some(proof_element) = proof.first() {
                    let mut tampered_proof = proof.clone();
                    let mut tampered_element = proof_element.clone().to_array();
                    tampered_element[tamper_idx] =
                        tampered_element[tamper_idx].wrapping_add(1);
                    tampered_proof.set(0, Bytes32::from_array(&e, &tampered_element));

                    prop_assert!(!Verifier::<Keccak256>::verify(&e, tampered_proof, root, leaf));
                }
            }
        )
    }
    #[test]
    fn proof_length_affects_verification() {
        let e = Env::default();
        // Turn off the CPU/memory budget for testing.
        e.cost_estimate().budget().reset_unlimited();

        proptest!(
            |((proof, root, leaf) in valid_merkle_proof(e.clone(), 0),
             extra_hash: [u8; 32])| {
                let extra_hash = Bytes32::from_array(&e, &extra_hash);
                let mut longer_proof = proof.clone();
                longer_proof.push_back(extra_hash);
                prop_assert!(!Verifier::<Keccak256>::verify(&e, longer_proof, root.clone(), leaf.clone()));

                if !proof.is_empty() {
                    let shorter_proof = &proof.slice(1..);
                    prop_assert!(!Verifier::<Keccak256>::verify(&e, shorter_proof.clone(), root.clone(), leaf.clone()));
                    let shorter_proof = &proof.slice(..proof.len() - 1);
                    prop_assert!(!Verifier::<Keccak256>::verify(&e, shorter_proof.clone(), root, leaf));
                }
            }
        )
    }

    #[test]
    fn zero_length_proof_with_matching_leaf_and_root() {
        let e = Env::default();

        let root = Bytes32::from_array(&e, &[0u8; 32]);
        let leaf = root.clone();
        assert!(Verifier::<Keccak256>::verify(&e, soroban_sdk::Vec::new(&e), root, leaf));
    }

    #[test]
    fn verifies_valid_proofs() {
        let e = Env::default();
        // ```js
        // const merkleTree = StandardMerkleTree.of(
        //   toElements('ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/='),
        //   ['string'],
        // );
        //
        // const root  = merkleTree.root;
        // const hash  = merkleTree.leafHash(['A']);
        // const proof = merkleTree.getProof(['A']);
        // ```
        let root = to_bytes!(e, "b89eb120147840e813a77109b44063488a346b4ca15686185cf314320560d3f3");
        let leaf_a =
            to_bytes!(e, "6efbf77e320741a027b50f02224545461f97cd83762d5fbfeb894b9eb3287c16");
        let leaf_b =
            to_bytes!(e, "7051e21dd45e25ed8c605a53da6f77de151dcbf47b0e3ced3c5d8b61f4a13dbc");
        let proof = to_bytes_array!(
            e,
            "7051e21dd45e25ed8c605a53da6f77de151dcbf47b0e3ced3c5d8b61f4a13dbc",
            "1629d3b5b09b30449d258e35bbd09dd5e8a3abb91425ef810dc27eef995f7490",
            "633d21baee4bbe5ed5c51ac0c68f7946b8f28d2937f0ca7ef5e1ea9dbda52e7a",
            "8a65d3006581737a3bab46d9e4775dbc1821b1ea813d350a13fcd4f15a8942ec",
            "d6c3f3e36cd23ba32443f6a687ecea44ebfe2b8759a62cccf7759ec1fb563c76",
            "276141cd72b9b81c67f7182ff8a550b76eb96de9248a3ec027ac048c79649115"
        );

        let verification =
            Verifier::<Keccak256>::verify(&e, proof.clone(), root.clone(), leaf_a.clone());
        assert!(verification);

        let no_such_leaf = commutative_hash_pair(&leaf_a, &leaf_b, Keccak256::new(&e));
        let verification = Verifier::<Keccak256>::verify(&e, proof.slice(1..), root, no_such_leaf);
        assert!(verification);
    }

    #[test]
    fn rejects_invalid_proofs() {
        // ```js
        // const correctMerkleTree = StandardMerkleTree.of(toElements('abc'), ['string']);
        // const otherMerkleTree = StandardMerkleTree.of(toElements('def'), ['string']);
        //
        // const root = correctMerkleTree.root;
        // const leaf = correctMerkleTree.leafHash(['a']);
        // const proof = otherMerkleTree.getProof(['d']);
        // ```
        let e = Env::default();
        let root = to_bytes!(e, "f2129b5a697531ef818f644564a6552b35c549722385bc52aa7fe46c0b5f46b1");
        let leaf = to_bytes!(e, "9c15a6a0eaeed500fd9eed4cbeab71f797cefcc67bfd46683e4d2e6ff7f06d1c");
        let proof =
            to_bytes!(e, "7b0c6cd04b82bfc0e250030a5d2690c52585e0cc6a4f3bc7909d7723b0236ece");

        let verification =
            Verifier::<Keccak256>::verify(&e, soroban_sdk::vec![&e, proof], root, leaf);
        assert!(!verification);
    }

    #[test]
    fn rejects_proofs_with_invalid_length() {
        // ```js
        // const merkleTree = StandardMerkleTree.of(toElements('abc'), ['string']);
        //
        // const root = merkleTree.root;
        // const leaf = merkleTree.leafHash(['a']);
        // const proof = merkleTree.getProof(['a']);
        // ```
        let e = Env::default();
        let root = to_bytes!(e, "f2129b5a697531ef818f644564a6552b35c549722385bc52aa7fe46c0b5f46b1");
        let leaf = to_bytes!(e, "9c15a6a0eaeed500fd9eed4cbeab71f797cefcc67bfd46683e4d2e6ff7f06d1c");
        let proof = to_bytes_array!(
            e,
            "19ba6c6333e0e9a15bf67523e0676e2f23eb8e574092552d5e888c64a4bb3681",
            "9cf5a63718145ba968a01c1d557020181c5b252f665cf7386d370eddb176517b"
        );

        let verification = Verifier::<Keccak256>::verify(&e, proof.slice(..1), root, leaf);
        assert!(!verification);
    }

    #[test]
    fn verify_empty_proof_should_mean_leaf_equal_to_root() {
        // ```js
        // const merkleTree = StandardMerkleTree.of(toElements('abc'), ['string']);
        //
        // const root = merkleTree.root;
        // const leaf = merkleTree.leafHash(['a']);
        // const proof = merkleTree.getProof(['a']);
        // ```
        let e = Env::default();
        let root = to_bytes!(e, "f2129b5a697531ef818f644564a6552b35c549722385bc52aa7fe46c0b5f46b1");
        let leaf = to_bytes!(e, "9c15a6a0eaeed500fd9eed4cbeab71f797cefcc67bfd46683e4d2e6ff7f06d1c");
        let proof = soroban_sdk::vec![&e];

        // valid if root == leaf
        assert!(Verifier::<Keccak256>::verify(&e, proof.clone(), root.clone(), root.clone()));

        // invalid if root != leaf
        assert!(!Verifier::<Keccak256>::verify(&e, proof, root, leaf));
    }
}
