use soroban_sdk::{contract, contractimpl, contracttype, token, Address, BytesN, Env, Vec};
use stellar_crypto::sha256::Sha256;
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
