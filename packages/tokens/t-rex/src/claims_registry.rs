use soroban_sdk::{Address, Env, Vec};

pub trait ClaimVerifier {
    // Checks if an account has valid claim issued by a given claim issuer
    fn is_claim_valid(e: &Env, account: Address, claim_topic: u32, claim_issuer: Address) -> bool;
}

pub trait ClaimsRegistry: ClaimVerifier {
    // Every claim issuer can be assigned specific topics.
    // Update with claim_topics = [] means issuer can't add claims any more.
    // TODO: enumerate claim_issuers and claim_topics
    fn update_issuer_claim_topics(
        e: &Env,
        claim_issuer: Address,
        claim_topics: Vec<u32>,
        agent: Address,
    );

    // claim_issuer must have persmissions for this claim_topic and must be authorized
    fn add_claim(e: &Env, account: Address, claim_topic: u32, claim_issuer: Address, data: Vec<u8>);

    // only account can remove
    fn remove_claim(e: &Env, account: Address, claim_topic: u32, claim_issuer: Address);

    // agent must be authorized and can be same as claim_issuer or has a manager role
    fn revoke_claim(
        e: &Env,
        account: Address,
        claim_topic: u32,
        claim_issuer: Address,
        agent: Address,
    );
}
