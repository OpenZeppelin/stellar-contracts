use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};
use stellar_governance::governor::{self as governor, Governor, ProposalState};

/// Storage key for the timelock contract address.
#[contracttype]
enum GovernorTimelockKey {
    Timelock,
    OperationID(BytesN<32>),
}

#[contract]
pub struct GovernorTimelockContract;

#[contractimpl]
impl GovernorTimelockContract {
    pub fn __constructor(
        e: &Env,
        token_contract: Address,
        timelock_contract: Address,
        voting_delay: u32,
        voting_period: u32,
        proposal_threshold: u128,
        quorum: u128,
    ) {
        governor::set_name(e, String::from_str(e, "GovernorTimelock"));
        governor::set_version(e, String::from_str(e, "1.0.0"));
        governor::set_token_contract(e, &token_contract);
        governor::set_voting_delay(e, voting_delay);
        governor::set_voting_period(e, voting_period);
        governor::set_proposal_threshold(e, proposal_threshold);
        governor::set_quorum(e, quorum);
        e.storage().instance().set(&GovernorTimelockKey::Timelock, &timelock_contract);
    }

    /// Returns the address of the timelock contract.
    pub fn timelock(e: &Env) -> Address {
        get_timelock(e)
    }
}

fn get_timelock(e: &Env) -> Address {
    e.storage().instance().get(&GovernorTimelockKey::Timelock).expect("timelock not set")
}

fn zero_predecessor(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[0u8; 32])
}

#[contractimpl(contracttrait)]
impl Governor for GovernorTimelockContract {
    /// Enables the queuing step in the proposal lifecycle.
    fn proposals_need_queuing(_e: &Env) -> bool {
        true
    }

    /// Queues a succeeded proposal: transitions governor state to `Queued`
    /// and schedules a timelock operation that will call back into
    /// `execute` after the delay.
    ///
    /// The timelock operation targets this governor contract itself:
    /// `target = governor, function = "execute"`. This way the timelock
    /// enforces the delay, and execution flows back through the governor
    /// which invokes the real targets via `storage::execute`.
    ///
    /// The `description_hash` is reused as the timelock operation salt,
    /// ensuring a unique timelock operation ID per proposal.
    fn queue(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        eta: u32,
        _operator: Address,
    ) -> BytesN<32> {
        let proposal_id =
            governor::hash_proposal(e, &targets, &functions, &args, &description_hash);
        let snapshot = governor::get_proposal_snapshot(e, &proposal_id);
        let quorum = Self::quorum(e, snapshot);
        let proposal_id = governor::queue(
            e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            &description_hash,
            eta,
            quorum,
        );

        // Schedule a timelock operation that calls governor.execute() after
        // the delay. The timelock becomes the executor.
        let timelock = get_timelock(e);
        let delay = eta.saturating_sub(e.ledger().sequence());

        // The timelock operation will call: governor.execute(targets,
        // functions, args, description_hash, timelock_address)
        let execute_args: Vec<Val> =
            (targets, functions, args, description_hash.clone(), timelock.clone()).into_val(e);

        let op_id = e.invoke_contract::<BytesN<32>>(
            &timelock,
            &Symbol::new(e, "schedule_op"),
            (
                e.current_contract_address(),
                Symbol::new(e, "execute"),
                execute_args,
                zero_predecessor(e),
                description_hash,
                delay,
                e.current_contract_address(),
            )
                .into_val(e),
        );
        e.storage()
            .persistent()
            .set(&GovernorTimelockKey::OperationID(proposal_id.clone()), &op_id);

        proposal_id
    }

    /// Executes a queued proposal by invoking the real target contracts.
    ///
    /// In the timelock integration, this function is called by the timelock
    /// after the delay has elapsed. Only the timelock can trigger execution.
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        executor: Address,
    ) -> BytesN<32> {
        // Only the timelock contract can call execute.
        let timelock = get_timelock(e);
        assert!(executor == timelock);
        executor.require_auth();

        let proposal_id =
            governor::hash_proposal(e, &targets, &functions, &args, &description_hash);
        let snapshot = governor::get_proposal_snapshot(e, &proposal_id);
        let quorum = Self::quorum(e, snapshot);
        governor::execute(
            e,
            targets,
            functions,
            args,
            &description_hash,
            Self::proposals_need_queuing(e),
            quorum,
        )
    }

    /// Restricted cancellation: only the original proposer can cancel.
    ///
    /// Cancelling at the governor level is sufficient — if someone later
    /// tries to trigger the timelock operation, `execute` will reject it
    /// because the proposal is no longer in the `Queued` state.
    fn cancel(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        operator: Address,
    ) -> BytesN<32> {
        let proposal_id =
            governor::hash_proposal(e, &targets, &functions, &args, &description_hash);
        let proposer = governor::get_proposal_proposer(e, &proposal_id);
        assert!(operator == proposer);
        operator.require_auth();

        // Cancel in timelock if it's been already queued.
        if let Some(op_id) = e
            .storage()
            .persistent()
            .get::<_, BytesN<32>>(&GovernorTimelockKey::OperationID(proposal_id))
        {
            let timelock = get_timelock(e);
            e.invoke_contract::<BytesN<32>>(
                &timelock,
                &Symbol::new(e, "cancel_op"),
                (op_id, operator).into_val(e),
            );
        }

        governor::cancel(e, targets, functions, args, &description_hash)
    }
}
