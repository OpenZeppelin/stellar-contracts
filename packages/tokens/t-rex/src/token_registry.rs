use soroban_sdk::{Address, Env, Vec};
use stellar_fungible::allowlist::FungibleAllowList;

pub trait IdentityVerifier {
    // Checks if a user address is verified according to the current requirements
    // This is the only function required by RWA tokens for transfer validation
    fn is_verified(e: &Env, account: Address) -> bool;
}

pub trait TokenTopics {
    // Adds a claim topic to the required topics
    fn add_claim_topic(e: &Env, claim_topic: u32, agent: Address);

    // Removes a claim topic from the required topics
    fn remove_claim_topic(e: &Env, claim_topic: u32, agent: Address);

    // Gets the list of required claim topics
    fn get_claim_topics(e: &Env) -> Vec<u32>;
}

pub trait TokenRegistry: IdentityVerifier + TokenTopics + FungibleAllowList {}
