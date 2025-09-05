use soroban_sdk::{testutils::Address as _, xdr::ToXdr, Address, BytesN, Env, Vec};
use stellar_contract_utils::crypto::{
    hashable::commutative_hash_pair, hasher::Hasher, sha256::Sha256,
};

use crate::contract::{MerkleVoting, MerkleVotingClient, VoteData};

fn hash_vote(e: &Env, data: &VoteData) -> BytesN<32> {
    let mut hasher = Sha256::new(e);
    hasher.update(data.clone().to_xdr(e));
    hasher.finalize()
}

#[test]
fn test_merkle_voting() {
    let e = Env::default();

    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);

    let vote1 = VoteData { index: 0, account: voter1.clone(), voting_power: 100 };
    let vote2 = VoteData { index: 1, account: voter2.clone(), voting_power: 50 };

    let leaf1 = hash_vote(&e, &vote1);
    let leaf2 = hash_vote(&e, &vote2);

    let root = commutative_hash_pair(&leaf1, &leaf2, Sha256::new(&e));

    let contract_id = e.register(MerkleVoting, (root,));
    let client = MerkleVotingClient::new(&e, &contract_id);

    let proof1 = Vec::from_array(&e, [leaf2.clone()]);
    client.vote(&vote1, &proof1, &true);

    let proof2 = Vec::from_array(&e, [leaf1.clone()]);
    client.vote(&vote2, &proof2, &false);

    // Verify votes were recorded
    assert!(client.has_voted(&0));
    assert!(client.has_voted(&1));

    // Check vote results
    let (votes_pro, votes_against) = client.get_vote_results();
    assert_eq!(votes_pro, 100);
    assert_eq!(votes_against, 50);
}
