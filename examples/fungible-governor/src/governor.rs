use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Symbol, Val, Vec};
use stellar_governance::governor::{
    storage::{self, set_token_contract},
    Governor, ProposalState,
};

#[contract]
pub struct GovernorContract;

#[contractimpl]
impl GovernorContract {
    pub fn __constructor(
        e: &Env,
        token_contract: Address,
        voting_delay: u32,
        voting_period: u32,
        proposal_threshold: u128,
        quorum: u128,
    ) {
        storage::set_name(e, String::from_str(e, "ExampleGovernor"));
        storage::set_version(e, String::from_str(e, "1.0.0"));
        set_token_contract(e, &token_contract);
        storage::set_voting_delay(e, voting_delay);
        storage::set_voting_period(e, voting_period);
        storage::set_proposal_threshold(e, proposal_threshold);
        storage::set_quorum(e, quorum);
    }
}

#[contractimpl(contracttrait)]
impl Governor for GovernorContract {}
