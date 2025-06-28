//! NOTE: The values used as input for these tests were all generated using
//! <https://github.com/OpenZeppelin/merkle-tree>.

extern crate std;

use std::format;

use hex_literal::hex;
use proptest::{prelude::*, prop_compose};
use soroban_sdk::Env;

use crate::{
    hashable::{commutative_hash_pair, hash_pair},
    hasher::Hasher,
    keccak::Keccak256,
    merkle::{Bytes32, Verifier},
    sha256::Sha256,
};

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

prop_compose! {
    fn valid_merkle_proof_with_index(e: Env, max_proof_len: usize)(
        leaf: [u8; 32],
        (proof, index) in prop::collection::vec(any::<[u8; 32]>(), 0..max_proof_len)
            .prop_flat_map(|proof| {
                let mut len = proof.len() as u32;
                if len == 0 { len = 1 } // to avoid (0..0) range error
                (
                    Just(proof.clone()),
                    (0..len).prop_filter("index can't exceed max leaves count", move |i| *i < 1 << len)
                )
            }),
    ) -> (soroban_sdk::Vec<Bytes32>, Bytes32, Bytes32, u32) {
        let mut current = Bytes32::from_array(&e, &leaf);
        let mut proof_vec: soroban_sdk::Vec<Bytes32> = soroban_sdk::Vec::new(&e);
        let mut current_index = index;
        for hash in &proof {
            let hash = Bytes32::from_array(&e, &hash.clone());
            proof_vec.push_back(hash.clone());
            current = if current_index.is_multiple_of(2) {
                hash_pair(&current, &hash, Keccak256::new(&e))
            } else {
                hash_pair(&hash, &current, Keccak256::new(&e))
            };
            current_index /= 2;
        }
        let root = current;
        let leaf = Bytes32::from_array(&e, &leaf);
        (proof_vec, root, leaf, index)
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
    let leaf_a = to_bytes!(e, "6efbf77e320741a027b50f02224545461f97cd83762d5fbfeb894b9eb3287c16");
    let leaf_b = to_bytes!(e, "7051e21dd45e25ed8c605a53da6f77de151dcbf47b0e3ced3c5d8b61f4a13dbc");
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
fn sha256_verifies_valid_proofs() {
    // generated with the same data as above but using sha256 as a hashing function
    let e = Env::default();
    let root = to_bytes!(e, "b0d388be1fe96067c100c6731770b70f87fa1287c4d6ddf9e107bdd015ae445c");
    let leaf_a = to_bytes!(e, "9c707ca1d6e1963a6a974a40f20c51f899cc96c9a7edef911953f424186641bf");
    let leaf_b = to_bytes!(e, "a2e125b5cc0b89c38c9030830083dc6d194e4d38d6d19aba7340d5a34767ddd2");

    let proof = to_bytes_array!(
        e,
        "a2e125b5cc0b89c38c9030830083dc6d194e4d38d6d19aba7340d5a34767ddd2",
        "074b68e00644cab3861ec0d5d4ff7deb3c65b2f12ac6025a18649331fcf50a8a",
        "f4c5a235a25aa2c56909fda65ece8f61b191ecf13a5aaaccff43bcbde3ec0821",
        "61a0ecbe2ce82a83da2e66315362fc300b63cdb922895a022c2842bd73d5f162",
        "c05b8b50e47b1583b14272de489eaadfeaf264e3a1b868be1f290f219549fc83",
        "ae9abdff40bea69fda504681175ffe68c109866f37e1eab183365d73c49c9939"
    );
    let verification = Verifier::<Sha256>::verify(&e, proof.clone(), root.clone(), leaf_a.clone());
    assert!(verification);

    let no_such_leaf = commutative_hash_pair(&leaf_a, &leaf_b, Sha256::new(&e));
    let verification = Verifier::<Sha256>::verify(&e, proof.slice(1..), root, no_such_leaf);
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
    let proof = to_bytes!(e, "7b0c6cd04b82bfc0e250030a5d2690c52585e0cc6a4f3bc7909d7723b0236ece");

    let verification = Verifier::<Keccak256>::verify(&e, soroban_sdk::vec![&e, proof], root, leaf);
    assert!(!verification);
}

#[test]
fn sha256_rejects_invalid_proofs() {
    // generated with the same data as above but using sha256 as a hashing function
    let e = Env::default();
    let root = to_bytes!(e, "d132af2ab11889f767680c257c81620b4678f10c014eec1c4711991ab7a02ab4");
    let leaf = to_bytes!(e, "b9e9db137d987ce376feabe4acc5ee8b23a2d460699cc8bd7e1fe001cbd99df0");
    let proof = to_bytes_array!(
        e,
        "0b623bf3b3a650a8072bc8b3001b2b74d7e63b43bf81beb332e536207b4a58e7",
        "ef53964d3736e523a79fe02137c6dba7d2b151fea57aa43c6f637514f2303f72"
    );

    let verification = Verifier::<Sha256>::verify(&e, proof, root, leaf);
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
fn sha256_rejects_proofs_with_invalid_length() {
    // generated with the same data as above but using sha256 as a hashing function
    let e = Env::default();
    let root = to_bytes!(e, "d132af2ab11889f767680c257c81620b4678f10c014eec1c4711991ab7a02ab4");
    let leaf = to_bytes!(e, "b9e9db137d987ce376feabe4acc5ee8b23a2d460699cc8bd7e1fe001cbd99df0");
    let proof = to_bytes_array!(
        e,
        "9a81c362fd809d46eb23f6920461ce343f9384bae29e11d005990fd2fbfb78c2",
        "ee8476cf31e3608c6ef618451476c5513ceef7f6d9f4af12df9fd4e4501210c3"
    );

    let verification = Verifier::<Sha256>::verify(&e, proof.slice(..1), root, leaf);
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

#[test]
fn sha256_verify_empty_proof_should_mean_leaf_equal_to_root() {
    // generated with the same data as above but using sha256 as a hashing function
    let e = Env::default();
    let root = to_bytes!(e, "d132af2ab11889f767680c257c81620b4678f10c014eec1c4711991ab7a02ab4");
    let leaf = to_bytes!(e, "b9e9db137d987ce376feabe4acc5ee8b23a2d460699cc8bd7e1fe001cbd99df0");
    let proof = soroban_sdk::vec![&e];

    // valid if root == leaf
    assert!(Verifier::<Keccak256>::verify(&e, proof.clone(), root.clone(), root.clone()));

    // invalid if root != leaf
    assert!(!Verifier::<Keccak256>::verify(&e, proof, root, leaf));
}

#[test]
fn proof_validates_with_index() {
    let e = Env::default();
    // Turn off the CPU/memory budget for testing.
    e.cost_estimate().budget().reset_unlimited();

    proptest!(
        |((proof, root, leaf, index) in valid_merkle_proof_with_index(e.clone(), 32))| {
            prop_assert!(Verifier::<Keccak256>::verify_with_index(&e, proof, root, leaf, index));
        }
    )
}

#[test]
#[should_panic(expected = "Error(Contract, #1400)")]
fn verify_fails_with_wrong_proof_len() {
    let e = Env::default();
    // root and leaf values are not important, as we try to test bound errors
    let root = Bytes32::from_array(&e, &[0u8; 32]);
    let leaf = Bytes32::from_array(&e, &[0u8; 32]);
    let mut proof = soroban_sdk::vec![&e];
    for _ in 0..32 {
        proof.push_back(leaf.clone());
    }
    let _ = Verifier::<Keccak256>::verify_with_index(&e, proof, root, leaf, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #1401)")]
fn verify_fails_with_wrong_index() {
    let e = Env::default();
    // root and leaf values are not important, as we try to test bound errors
    let root = Bytes32::from_array(&e, &[0u8; 32]);
    let leaf = Bytes32::from_array(&e, &[0u8; 32]);
    let mut proof = soroban_sdk::vec![&e];
    let wrong_idx = 1 << 31;
    for _ in 0..31 {
        proof.push_back(leaf.clone());
    }
    let _ = Verifier::<Keccak256>::verify_with_index(&e, proof, root, leaf, wrong_idx);
}
