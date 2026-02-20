extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Ledger},
    vec,
    xdr::ToXdr,
    Address, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};
use stellar_governance::governor::ProposalState;

use crate::{GovernorContract, GovernorContractClient, TokenContract, TokenContractClient};

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

// ==================== Helpers ====================

struct TestSetup<'a> {
    e: Env,
    token: TokenContractClient<'a>,
    governor: GovernorContractClient<'a>,
    target: TargetContractClient<'a>,
}

fn setup() -> TestSetup<'static> {
    let e = Env::default();
    e.mock_all_auths();

    // Start at ledger 100 so that snapshot = sequence - 1 never underflows
    e.ledger().set_sequence_number(100);

    let owner = Address::generate(&e);

    let token_address = e.register(TokenContract, (owner.clone(),));
    let token = TokenContractClient::new(&e, &token_address);

    let governor_address = e.register(
        GovernorContract,
        (token_address.clone(), VOTING_DELAY, VOTING_PERIOD, PROPOSAL_THRESHOLD, QUORUM),
    );
    let governor = GovernorContractClient::new(&e, &governor_address);

    let target_address = e.register(TargetContract, ());
    let target = TargetContractClient::new(&e, &target_address);

    TestSetup { e, token, governor, target }
}

/// Mints tokens to `account` and self-delegates so voting power is recorded.
fn mint_and_delegate(setup: &TestSetup, account: &Address, amount: i128) {
    setup.token.mint(account, &amount);
    setup.token.delegate(account, account);
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
    let description = String::from_str(e, "Set value proposal");
    (targets, functions, args, description)
}

/// Hashes the description to produce the description_hash used for
/// execute/cancel.
fn description_hash(e: &Env, description: &String) -> BytesN<32> {
    e.crypto().keccak256(&description.clone().to_xdr(e)).to_bytes()
}

// ==================== Tests ====================

#[test]
fn initialization() {
    let s = setup();

    assert_eq!(s.governor.name(), String::from_str(&s.e, "ExampleGovernor"));
    assert_eq!(s.governor.version(), String::from_str(&s.e, "1.0.0"));
    assert_eq!(s.governor.voting_delay(), VOTING_DELAY);
    assert_eq!(s.governor.voting_period(), VOTING_PERIOD);
    assert_eq!(s.governor.proposal_threshold(), PROPOSAL_THRESHOLD);
    assert_eq!(s.governor.quorum(&0), QUORUM);
    assert_eq!(s.governor.get_token_contract(), s.token.address);
}

#[test]
fn token_minting_and_delegation() {
    let s = setup();
    let voter = Address::generate(&s.e);

    s.token.mint(&voter, &1000);
    assert_eq!(s.token.balance(&voter), 1000);

    // Before delegation, no voting power
    assert_eq!(s.token.get_votes(&voter), 0);

    // Self-delegate
    s.token.delegate(&voter, &voter);
    assert_eq!(s.token.get_votes(&voter), 1000);
}

#[test]
fn propose_and_query_state() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    mint_and_delegate(&s, &proposer, 1000);

    // Advance ledger so the checkpoint at ledger 100 is in the past
    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Proposal should be Pending
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Pending);

    // Verify snapshot and deadline
    // vote_start = 200 + VOTING_DELAY = 210
    // vote_end = 210 + VOTING_PERIOD = 310
    assert_eq!(s.governor.proposal_snapshot(&proposal_id), 210);
    assert_eq!(s.governor.proposal_deadline(&proposal_id), 310);
    assert_eq!(s.governor.proposal_proposer(&proposal_id), proposer);
}

#[test]
fn full_governance_lifecycle() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter1 = Address::generate(&s.e);
    let voter2 = Address::generate(&s.e);

    // Mint and delegate at ledger 100
    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter1, 400);
    mint_and_delegate(&s, &voter2, 300);

    // Advance to ledger 200 to propose
    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // State: Pending
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Pending);

    // Advance past vote_start (210) -> Active
    s.e.ledger().set_sequence_number(211);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Active);

    // Cast votes: voter1 For (1), voter2 For (1)
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, "I support this"), &voter1);
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, "Agreed"), &voter2);

    // Verify has_voted
    assert!(s.governor.has_voted(&proposal_id, &voter1));
    assert!(s.governor.has_voted(&proposal_id, &voter2));
    assert!(!s.governor.has_voted(&proposal_id, &proposer));

    // Advance past vote_end (310) -> Succeeded
    // Total For votes: 400 + 300 = 700 > quorum of 500
    s.e.ledger().set_sequence_number(311);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Succeeded);

    // Execute the proposal
    s.governor.execute(&targets, &functions, &args, &desc_hash, &proposer);

    // Verify target contract was updated
    assert_eq!(s.target.get_value(), 42);

    // State: Executed
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Executed);
}

#[test]
fn proposal_defeated_when_quorum_not_reached() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter, 100); // Only 100, quorum is 500

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Advance to Active
    s.e.ledger().set_sequence_number(211);

    // Vote For, but only 100 voting power
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, ""), &voter);

    // Advance past voting period -> Defeated because quorum not reached
    s.e.ledger().set_sequence_number(311);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Defeated);
}

#[test]
fn proposal_defeated_when_against_votes_win() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter_for = Address::generate(&s.e);
    let voter_against = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter_for, 300);
    mint_and_delegate(&s, &voter_against, 600);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    s.e.ledger().set_sequence_number(211);

    // voter_for votes For (1), voter_against votes Against (0)
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, ""), &voter_for);
    s.governor.cast_vote(&proposal_id, &0, &String::from_str(&s.e, ""), &voter_against);

    // Advance past voting -> Defeated because Against > For
    s.e.ledger().set_sequence_number(311);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Defeated);
}

#[test]
fn cancel_proposal() {
    let s = setup();
    let proposer = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Cancel while Pending
    s.governor.cancel(&targets, &functions, &args, &desc_hash, &proposer);

    // State: Canceled
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Canceled);
}

#[test]
#[should_panic(expected = "#5006")]
fn execute_fails_when_defeated() {
    let s = setup();
    let proposer = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Advance past voting with no votes -> Defeated
    s.e.ledger().set_sequence_number(311);

    // Should panic with ProposalNotSuccessful = 5006
    s.governor.execute(&targets, &functions, &args, &desc_hash, &proposer);
}

#[test]
#[should_panic(expected = "#5002")]
fn propose_fails_with_insufficient_voting_power() {
    let s = setup();
    let proposer = Address::generate(&s.e);

    // Only 50 tokens, threshold is 100
    mint_and_delegate(&s, &proposer, 50);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    // Should panic with InsufficientProposerVotes = 5002
    s.governor.propose(&targets, &functions, &args, &description, &proposer);
}

#[test]
fn abstain_votes_count_toward_quorum() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter_for = Address::generate(&s.e);
    let voter_abstain = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter_for, 200);
    mint_and_delegate(&s, &voter_abstain, 400);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    s.e.ledger().set_sequence_number(211);

    // voter_for votes For (1), voter_abstain votes Abstain (2)
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, ""), &voter_for);
    s.governor.cast_vote(&proposal_id, &2, &String::from_str(&s.e, ""), &voter_abstain);

    // Quorum = 500, For + Abstain = 200 + 400 = 600 >= 500
    // Tally: For (200) > Against (0)
    s.e.ledger().set_sequence_number(311);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Succeeded);

    // Execute
    s.governor.execute(&targets, &functions, &args, &desc_hash, &proposer);
    assert_eq!(s.target.get_value(), 42);
}

#[test]
fn multiple_proposals() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter, 600);

    s.e.ledger().set_sequence_number(200);

    // Create two different proposals
    let (targets1, functions1, args1, desc1) = build_proposal(&s.e, &s.target.address, 42);
    let (targets2, functions2, args2, desc2) = {
        let targets = vec![&s.e, s.target.address.clone()];
        let functions = vec![&s.e, symbol_short!("set_value")];
        let args: Vec<Vec<Val>> = vec![&s.e, vec![&s.e, 99u32.into_val(&s.e)]];
        let description = String::from_str(&s.e, "Set value to 99");
        (targets, functions, args, description)
    };

    let desc_hash1 = description_hash(&s.e, &desc1);
    let desc_hash2 = description_hash(&s.e, &desc2);

    let proposal1 = s.governor.propose(&targets1, &functions1, &args1, &desc1, &proposer);
    let proposal2 = s.governor.propose(&targets2, &functions2, &args2, &desc2, &proposer);

    assert_ne!(proposal1, proposal2);

    // Advance to Active
    s.e.ledger().set_sequence_number(211);

    // Vote For on proposal 1, Against on proposal 2
    s.governor.cast_vote(&proposal1, &1, &String::from_str(&s.e, ""), &voter);
    s.governor.cast_vote(&proposal2, &0, &String::from_str(&s.e, ""), &voter);

    // Advance past voting
    s.e.ledger().set_sequence_number(311);

    // Proposal 1: Succeeded (For: 600, quorum met)
    assert_eq!(s.governor.proposal_state(&proposal1), ProposalState::Succeeded);
    // Proposal 2: Defeated (Against: 600, For: 0)
    assert_eq!(s.governor.proposal_state(&proposal2), ProposalState::Defeated);

    // Execute proposal 1
    s.governor.execute(&targets1, &functions1, &args1, &desc_hash1, &proposer);
    assert_eq!(s.target.get_value(), 42);

    // Executing proposal 2 should fail
    let result = s.governor.try_execute(&targets2, &functions2, &args2, &desc_hash2, &proposer);
    assert!(result.is_err());
}

#[test]
fn delegation_to_another_account() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let token_holder = Address::generate(&s.e);
    let delegatee = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);

    // token_holder has tokens but delegates to delegatee
    s.token.mint(&token_holder, &600);
    s.token.delegate(&token_holder, &delegatee);

    // delegatee has 600 voting power, token_holder has 0
    assert_eq!(s.token.get_votes(&delegatee), 600);
    assert_eq!(s.token.get_votes(&token_holder), 0);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let desc_hash = description_hash(&s.e, &description);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    s.e.ledger().set_sequence_number(211);

    // delegatee votes with the delegated power
    s.governor.cast_vote(
        &proposal_id,
        &1,
        &String::from_str(&s.e, "Voting with delegated power"),
        &delegatee,
    );

    s.e.ledger().set_sequence_number(311);
    assert_eq!(s.governor.proposal_state(&proposal_id), ProposalState::Succeeded);

    s.governor.execute(&targets, &functions, &args, &desc_hash, &proposer);
    assert_eq!(s.target.get_value(), 42);
}

#[test]
fn voting_power_snapshot_at_proposal_creation() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter, 600);

    // Advance and propose at ledger 200
    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Advance to Active (past vote_start 210)
    s.e.ledger().set_sequence_number(211);

    // Mint MORE tokens to voter AFTER vote_start (at ledger 211)
    // This should NOT affect their voting power for this proposal
    // because the snapshot is at vote_start (ledger 210)
    s.token.mint(&voter, &10000);

    // voter's weight should be based on snapshot at vote_start (210), which was 600
    let weight = s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, ""), &voter);
    assert_eq!(weight, 600);
}

#[test]
#[should_panic(expected = "#5005")]
fn cannot_vote_before_voting_starts() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter, 600);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Try to vote while still Pending (ledger 200 <= vote_start 210)
    // Should panic with ProposalNotActive = 5005
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, ""), &voter);
}

#[test]
#[should_panic(expected = "#5005")]
fn cannot_vote_after_voting_ends() {
    let s = setup();
    let proposer = Address::generate(&s.e);
    let voter = Address::generate(&s.e);

    mint_and_delegate(&s, &proposer, 200);
    mint_and_delegate(&s, &voter, 600);

    s.e.ledger().set_sequence_number(200);

    let (targets, functions, args, description) = build_proposal(&s.e, &s.target.address, 42);
    let proposal_id = s.governor.propose(&targets, &functions, &args, &description, &proposer);

    // Advance past voting end
    s.e.ledger().set_sequence_number(311);

    // Should panic with ProposalNotActive = 5005
    s.governor.cast_vote(&proposal_id, &1, &String::from_str(&s.e, ""), &voter);
}
