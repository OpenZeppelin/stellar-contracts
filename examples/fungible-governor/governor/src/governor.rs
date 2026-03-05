use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Symbol, Val, Vec};
use stellar_governance::governor::{self as governor, Governor, ProposalState};

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
        governor::set_name(e, String::from_str(e, "ExampleGovernor"));
        governor::set_version(e, String::from_str(e, "1.0.0"));
        governor::set_token_contract(e, &token_contract);
        governor::set_voting_delay(e, voting_delay);
        governor::set_voting_period(e, voting_period);
        governor::set_proposal_threshold(e, proposal_threshold);
        governor::set_quorum(e, quorum);
    }
}

#[contractimpl(contracttrait)]
impl Governor for GovernorContract {
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        executor: Address,
    ) -> BytesN<32> {
        // Open execution: any account can trigger a succeeded proposal,
        // as long as they authenticate themselves as `executor`.
        executor.require_auth();
        governor::execute(
            e,
            targets,
            functions,
            args,
            &description_hash,
            Self::proposal_needs_queuing(e),
        )
    }

    fn cancel(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        operator: Address,
    ) -> BytesN<32> {
        // Restricted cancellation: only the original proposer can cancel.
        let proposal_id =
            governor::hash_proposal(e, &targets, &functions, &args, &description_hash);
        let proposer = governor::get_proposal_proposer(e, &proposal_id);
        assert!(operator == proposer);
        operator.require_auth();
        governor::cancel(e, targets, functions, args, &description_hash)
    }
}
