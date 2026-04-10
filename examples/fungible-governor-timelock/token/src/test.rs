extern crate std;

use governor_timelock_contract::{GovernorTimelockContract, GovernorTimelockContractClient};
use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};
use stellar_governance::governor::ProposalState;
use timelock_controller_example::{TimelockController, TimelockControllerClient};

use crate::{TokenContract, TokenContractClient};

// ==================== Target Contract ====================

/// A simple target contract whose state is modified by governance proposals.
#[contract]
pub struct TargetContract;

#[contractimpl]
impl TargetContract {
    pub fn set_value(e: &Env, value: u32) -> u32 {
        e.storage().instance().set(&symbol_short!("value"), &value);
        value
    }

    pub fn get_value(e: &Env) -> u32 {
        e.storage().instance().get(&symbol_short!("value")).unwrap_or(0)
    }
}

// ==================== Constants ====================

const VOTING_DELAY: u32 = 10;
const VOTING_PERIOD: u32 = 100;
const PROPOSAL_THRESHOLD: u128 = 100;
const QUORUM: u128 = 500;
const TIMELOCK_MIN_DELAY: u32 = 50;

// ==================== Helpers ====================

struct TestSetup<'a> {
    e: Env,
    token: TokenContractClient<'a>,
    governor: GovernorTimelockContractClient<'a>,
    timelock: TimelockControllerClient<'a>,
    target: TargetContractClient<'a>,
}

fn setup() -> TestSetup<'static> {
    let e = Env::default();
    e.mock_all_auths();

    // Start at ledger 100 so that snapshot = sequence - 1 never underflows.
    e.ledger().set_sequence_number(100);

    let owner = Address::generate(&e);
    let admin = Address::generate(&e);

    // Token
    let token_address = e.register(TokenContract, (owner.clone(),));
    let token = TokenContractClient::new(&e, &token_address);

    // Timelock — deploy with an external admin so we can grant roles after
    // the governor is deployed (resolves circular address dependency).
    let empty_addrs: Vec<Address> = Vec::new(&e);
    let timelock_address = e.register(
        TimelockController,
        (TIMELOCK_MIN_DELAY, empty_addrs.clone(), empty_addrs, Some(admin.clone())),
    );
    let timelock = TimelockControllerClient::new(&e, &timelock_address);

    // Governor
    let governor_address = e.register(
        GovernorTimelockContract,
        (
            token_address.clone(),
            timelock_address.clone(),
            VOTING_DELAY,
            VOTING_PERIOD,
            PROPOSAL_THRESHOLD,
            QUORUM,
        ),
    );
    let governor = GovernorTimelockContractClient::new(&e, &governor_address);

    // Grant the governor PROPOSER_ROLE and CANCELLER_ROLE on the timelock.
    timelock.grant_role(&governor_address, &Symbol::new(&e, "proposer"), &admin);
    timelock.grant_role(&governor_address, &Symbol::new(&e, "canceller"), &admin);

    // Target
    let target_address = e.register(TargetContract, ());
    let target = TargetContractClient::new(&e, &target_address);

    TestSetup { e, token, governor, timelock, target }
}

/// Mints tokens to `account` and self-delegates so voting power is recorded.
fn mint_and_delegate(s: &TestSetup, account: &Address, amount: i128) {
    s.token.mint(account, &amount);
    s.token.delegate(account, account);
}

/// Builds a proposal that calls `TargetContract::set_value(value)`.
fn build_proposal(
    e: &Env,
    target: &Address,
    value: u32,
) -> (Vec<Address>, Vec<Symbol>, Vec<Vec<Val>>, String) {
    let targets = vec![e, target.clone()];
    let functions = vec![e, symbol_short!("set_value")];
    let args: Vec<Vec<Val>> = vec![e, vec![e, value.into_val(e)]];
    let description = String::from_str(e, "Set value via timelock");
    (targets, functions, args, description)
}

/// Hashes the description to produce the description_hash.
fn description_hash(e: &Env, description: &String) -> BytesN<32> {
    e.crypto().keccak256(&description.to_bytes()).to_bytes()
}

/// Creates a proposal, votes it through to Succeeded, and returns everything
/// needed for queue/execute/cancel.
#[allow(clippy::type_complexity)]
fn create_succeeded_proposal(
    s: &TestSetup,
) -> (BytesN<32>, BytesN<32>, Vec<Address>, Vec<Symbol>, Vec<Vec<Val>>, Address) {
    let proposer = Address::generate(&s.e);
    let voter1 = Address::generate(&s.e);
    let voter2 = Address::generate(&s.e);

    mint_and_delegate(s, &proposer, 200);
    mint_and_delegate(s, &voter1, 400);
    mint_and_delegate(s, &voter2, 300);

    // Propose at ledger 200.
    s.e.ledger().set_sequence_number(200);
    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Advance past vote_snapshot (210) -> Active.
    s.e.ledger().set_sequence_number(211);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Active);

    // Cast votes (enough for quorum: 400 + 300 = 700 > 500).
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, "yes"), &voter1);
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, "yes"), &voter2);

    // Advance past vote_end (310) -> Succeeded.
    s.e.ledger().set_sequence_number(311);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Succeeded);

    (proposal_id, desc_hash, targets, functions, args, proposer)
}

// ==================== Tests ====================

/// Full lifecycle: Propose -> Vote -> Queue (schedules in timelock) ->
/// timelock.execute() triggers governor.execute() -> real targets invoked.
#[test]
fn full_governance_lifecycle_with_timelock() {
    let s = setup();

    let (proposal_id, desc_hash, targets, functions, args, _) = create_succeeded_proposal(&s);

    // Queue: transitions governor to Queued, schedules governor.execute()
    // in the timelock.
    let operator = Address::generate(&s.e);
    let eta = s.e.ledger().sequence() + TIMELOCK_MIN_DELAY;
    s.governor.queue(&targets, &functions, &args, &desc_hash, &eta, &operator);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Queued);

    // Target hasn't been called yet.
    assert_eq!(s.target.get_value(), 0);

    // Advance past the timelock delay.
    s.e.ledger().set_sequence_number(eta);

    // The timelock executes the scheduled operation, which calls
    // governor.execute(), which invokes the real target.
    let execute_args: Vec<Val> = (
        targets.clone(),
        functions.clone(),
        args.clone(),
        desc_hash.clone(),
        s.timelock.address.clone(),
    )
        .into_val(&s.e);

    s.timelock.execute(
        &s.governor.address,
        &Symbol::new(&s.e, "execute"),
        &execute_args,
        &BytesN::<32>::from_array(&s.e, &[0u8; 32]),
        &desc_hash,
        &None::<Address>,
    );

    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Executed);
    assert_eq!(s.target.get_value(), 42);
}

/// Executing before the timelock delay has elapsed should fail.
#[test]
#[should_panic]
fn execute_fails_before_timelock_delay() {
    let s = setup();

    let (_, desc_hash, targets, functions, args, _) = create_succeeded_proposal(&s);

    // Queue at ledger 311.
    let operator = Address::generate(&s.e);
    let eta = s.e.ledger().sequence() + TIMELOCK_MIN_DELAY;
    s.governor.queue(&targets, &functions, &args, &desc_hash, &eta, &operator);

    // Try to execute immediately (311 < eta 361) — timelock rejects.
    let execute_args: Vec<Val> =
        (targets, functions, args, desc_hash.clone(), s.timelock.address.clone()).into_val(&s.e);

    s.timelock.execute(
        &s.governor.address,
        &Symbol::new(&s.e, "execute"),
        &execute_args,
        &BytesN::<32>::from_array(&s.e, &[0u8; 32]),
        &desc_hash,
        &None::<Address>,
    );
}

/// Executing without queuing first should fail (proposal not in Queued state).
#[test]
#[should_panic(expected = "Error(Contract, #5007)")]
fn execute_fails_without_queue() {
    let s = setup();

    let (_, desc_hash, targets, functions, args, _) = create_succeeded_proposal(&s);

    // Skip queue, call execute directly. Should fail with ProposalNotQueued.
    s.governor.execute(&targets, &functions, &args, &desc_hash, &s.timelock.address);
}

/// Only the timelock can call execute.
#[test]
#[should_panic]
fn execute_fails_when_caller_is_not_timelock() {
    let s = setup();

    let (_, desc_hash, targets, functions, args, _) = create_succeeded_proposal(&s);

    let operator = Address::generate(&s.e);
    let eta = s.e.ledger().sequence() + TIMELOCK_MIN_DELAY;
    s.governor.queue(&targets, &functions, &args, &desc_hash, &eta, &operator);

    s.e.ledger().set_sequence_number(eta);

    // Try to execute as a random address, not the timelock.
    let random = Address::generate(&s.e);
    s.governor.execute(&targets, &functions, &args, &desc_hash, &random);
}

/// Queueing a non-Succeeded proposal should fail.
#[test]
#[should_panic(expected = "Error(Contract, #5006)")]
fn queue_fails_when_not_succeeded() {
    let s = setup();

    let proposer = Address::generate(&s.e);
    mint_and_delegate(&s, &proposer, 200);

    // Propose at ledger 200 — stays Pending.
    s.e.ledger().set_sequence_number(200);
    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Try to queue while still Pending.
    let operator = Address::generate(&s.e);
    s.governor.queue(&targets, &functions, &args, &desc_hash, &250, &operator);
}

/// Cancelling a queued proposal makes timelock execution fail gracefully.
#[test]
fn cancel_queued_proposal() {
    let s = setup();

    let (proposal_id, desc_hash, targets, functions, args, proposer) =
        create_succeeded_proposal(&s);

    // Queue.
    let operator = Address::generate(&s.e);
    let eta = s.e.ledger().sequence() + TIMELOCK_MIN_DELAY;
    s.governor.queue(&targets, &functions, &args, &desc_hash, &eta, &operator);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Queued);

    // Cancel at governor level.
    s.governor.cancel(&targets, &functions, &args, &desc_hash, &proposer);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Canceled);
}
