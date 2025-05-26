use soroban_sdk::{
    contract, contractimpl, contracttype, token, xdr::ToXdr, Address, BytesN, Env, Vec,
};
use stellar_crypto::{hasher::Hasher, sha256::Sha256};
use stellar_merkle_distributor::MerkleDistributor;

type Distributor = MerkleDistributor<Sha256>;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    TokenAddress,
}

#[contracttype]
#[derive(Clone, Debug)]
struct Receiver {
    pub index: u32,
    pub address: Address,
    pub amount: i128,
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

    pub fn is_claimed(e: &Env, index: u32, receiver: Address, amount: i128) -> bool {
        let data = Receiver { index, address: receiver.clone(), amount };

        let mut hasher = Sha256::new(e);
        hasher.update(data.to_xdr(e));
        let leaf = hasher.finalize();

        Distributor::is_claimed(e, leaf)
    }

    pub fn is_claimed_by_hash(e: &Env, hash: BytesN<32>) -> bool {
        Distributor::is_claimed(e, hash)
    }

    pub fn claim(e: &Env, index: u32, receiver: Address, amount: i128, proof: Vec<BytesN<32>>) {
        let data = Receiver { index, address: receiver.clone(), amount };

        let mut hasher = Sha256::new(e);
        hasher.update(data.to_xdr(e));
        let leaf = hasher.finalize();

        Distributor::verify_and_set_claimed(e, leaf, proof);

        let token = e.storage().instance().get::<_, Address>(&DataKey::TokenAddress).unwrap();

        token::TokenClient::new(e, &token).transfer(
            &e.current_contract_address(),
            &receiver,
            &amount,
        );
    }
}
