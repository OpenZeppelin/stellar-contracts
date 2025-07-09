use soroban_sdk::{Address, Bytes, Env};

pub trait ClaimIssuer {
    // Checks if an identity has valid claim issued by a given claim issuer
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        signature: Bytes,
        data: Bytes,
    ) -> bool;
}

pub trait ExampleClaimIssuer: ClaimIssuer {
    fn add_claim(e: &Env, identity: Address, claim_topic: u32, signature: Bytes, data: Bytes);

    fn remove_claim(e: &Env, identity: Address, claim_topic: u32);

    // agent must be authorized and can be same as claim_issuer or has a manager role
    fn revoke_claim(e: &Env, identity: Address, claim_topic: u32, agent: Address);
}
