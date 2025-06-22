use soroban_sdk::{Address, Env, Vec};

pub trait ClaimTopics {
    // Adds a claim topic to the required topics
    fn add_claim_topic(operator: Address, claim_topic: u32);

    // Removes a claim topic from the required topics
    fn remove_claim_topic(operator: Address, claim_topic: u32);

    // Gets the list of required claim topics
    fn get_claim_topics() -> Vec<u32>;
}
