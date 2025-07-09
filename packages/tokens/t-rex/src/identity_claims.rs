use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, String, Vec};

pub type Bytes32 = BytesN<32>;

#[contracttype]
pub struct Claim {
    claim_topic: u32,
    scheme: u32,
    issuer: Address,
    signature: Bytes,
    data: Bytes,
    uri: String,
}

pub trait IdentityClaims {
    // returns claim_id
    #[allow(clippy::too_many_arguments)]
    fn add_claim(
        e: &Env,
        claim_topic: u32,
        scheme: u32,
        issuer: Address,
        signature: Bytes,
        data: Bytes,
        uri: String,
        operator: Address,
    ) -> Bytes32;

    fn get_claim(e: &Env, claim_id: Bytes32) -> Claim;

    fn get_claim_ids_by_topic(e: &Env, claim_topic: u32) -> Vec<Bytes32>;
}
