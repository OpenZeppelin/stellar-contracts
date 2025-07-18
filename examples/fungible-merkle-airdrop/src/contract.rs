//! # Airdrop Distributor for a Fungible Token
//!
//! This contract implements a Merkle-proof-based distribution mechanism for
//! fungible token airdrops.
//!
//! Participants can claim their airdrop by providing a valid Merkle proof
//! corresponding to a pre-generated Merkle tree. The claim is verified against
//! the Merkle root stored in the contract.
//!
//! Each leaf of the Merkle tree must be structured as a `Receiver` struct,
//! which includes:
//!
//! - `index: u32` — a unique index for identifying the claim.
//! - `address: Address` — the Stellar address eligible to claim.
//! - `amount: i128` — the amount of tokens allocated to the address.
//!
//! The contract and test logic were adapted from
//! [philipliu/soroban-merkle-airdrop](https://github.com/philipliu/soroban-merkle-airdrop).

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, BytesN, Env, Vec};
use stellar_contract_utils::crypto::sha256::Sha256;
use stellar_contract_utils::merkle_distributor::{IndexableLeaf, MerkleDistributor};

type Distributor = MerkleDistributor<Sha256>;

#[contracttype]
enum DataKey {
    TokenAddress,
}

#[contracttype]
struct Receiver {
    pub index: u32,
    pub address: Address,
    pub amount: i128,
}

impl IndexableLeaf for Receiver {
    fn index(&self) -> u32 {
        self.index
    }
}

#[contract]
pub struct AirdropContract;

#[contractimpl]
impl AirdropContract {
    pub fn __constructor(
        e: Env,
        root_hash: BytesN<32>,
        token: Address,
        funding_amount: i128,
        funding_source: Address,
    ) {
        Distributor::set_root(&e, root_hash);
        e.storage().instance().set(&DataKey::TokenAddress, &token);
        token::TokenClient::new(&e, &token).transfer(
            &funding_source,
            &e.current_contract_address(),
            &funding_amount,
        );
    }

    pub fn is_claimed(e: &Env, index: u32) -> bool {
        Distributor::is_claimed(e, index)
    }

    pub fn claim(e: &Env, index: u32, receiver: Address, amount: i128, proof: Vec<BytesN<32>>) {
        let data = Receiver { index, address: receiver.clone(), amount };
        Distributor::verify_and_set_claimed(e, data, proof);

        let token = e.storage().instance().get::<_, Address>(&DataKey::TokenAddress).unwrap();
        token::TokenClient::new(e, &token).transfer(
            &e.current_contract_address(),
            &receiver,
            &amount,
        );
    }
}
