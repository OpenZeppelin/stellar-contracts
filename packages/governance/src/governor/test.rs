use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events, Ledger},
    vec, Address, BytesN, Env, IntoVal, String, Symbol, TryFromVal, Val, Vec,
};

use crate::governor::{
    cancel, cast_vote, count_vote, execute, get_proposal_deadline, get_proposal_proposer,
    get_proposal_snapshot, get_proposal_state, get_proposal_threshold, get_proposal_vote_counts,
    get_quorum, get_voting_delay, get_voting_period, has_voted, hash_proposal, propose, queue,
    quorum_reached, set_name, set_proposal_threshold, set_quorum, set_token_contract, set_version,
    set_voting_delay, set_voting_period, tally_succeeded, Governor, GovernorQueries,
    GovernorSettings, ProposalCore, ProposalSettings, ProposalState, ProposalStorageKey,
    VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR,
};

#[contract]
pub(crate) struct MockContract;

/// A mock token contract that implements `get_votes_at_checkpoint`.
/// The voting power to return is stored under the `"power"` key.
#[contract]
pub(crate) struct MockTokenContract;

#[contractimpl]
impl MockTokenContract {
    pub fn get_votes_at_checkpoint(e: &Env, _account: Address, _ledger: u32) -> u128 {
        e.storage().instance().get(&Symbol::new(e, "power")).unwrap_or(0)
    }
}

pub(crate) fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
}

/// Sets up a governor contract with a mock token contract.
/// Returns (env, governor_address, token_address).
pub(crate) fn setup_env_with_token() -> (Env, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    // Start at ledger 100 so that `propose()` can safely compute
    // `snapshot = sequence - 1` without underflow.
    e.ledger().set_sequence_number(100);
    let contract_address = e.register(MockContract, ());
    let token_address = e.register(MockTokenContract, ());
    e.as_contract(&contract_address, || {
        set_token_contract(&e, &token_address);
    });
    (e, contract_address, token_address)
}

/// Builds the [`ProposalSettings`] from the stored configuration, the same
/// values the default `Governor::propose` passes when no getter is overridden.
pub(crate) fn stored_settings(e: &Env) -> ProposalSettings {
    ProposalSettings {
        threshold: get_proposal_threshold(e),
        voting_delay: get_voting_delay(e),
        voting_period: get_voting_period(e),
    }
}

/// Returns the proposal state exactly as the default
/// `GovernorQueries::proposal_state` computes it.
pub(crate) fn default_state(e: &Env, proposal_id: &BytesN<32>) -> ProposalState {
    let quorum = get_quorum(e, get_proposal_snapshot(e, proposal_id));
    let succeeded = quorum_reached(e, proposal_id, quorum) && tally_succeeded(e, proposal_id);
    get_proposal_state(e, proposal_id, succeeded)
}

/// Sets the voting power the mock token contract will return.
pub(crate) fn set_mock_voting_power(e: &Env, token_address: &Address, power: u128) {
    e.as_contract(token_address, || {
        e.storage().instance().set(&Symbol::new(e, "power"), &power);
    });
}

/// Initializes a governor with standard config for proposal tests.
pub(crate) fn setup_governor_config(e: &Env, contract_address: &Address) {
    e.as_contract(contract_address, || {
        set_name(e, String::from_str(e, "TestGov"));
        set_version(e, String::from_str(e, "1.0.0"));
        set_proposal_threshold(e, 100);
        set_voting_delay(e, 10);
        set_voting_period(e, 100);
        set_quorum(e, 50);
    });
}

/// Creates a simple single-action proposal parameter set.
pub(crate) fn simple_proposal(e: &Env) -> (Vec<Address>, Vec<Symbol>, Vec<Vec<Val>>, String) {
    let target = Address::generate(e);
    let targets = vec![e, target];
    let functions = vec![e, Symbol::new(e, "do_something")];
    let args: Vec<Vec<Val>> = vec![e, vec![e, 42u32.into_val(e)]];
    let description = String::from_str(e, "Test proposal");
    (targets, functions, args, description)
}

pub(crate) fn proposal_id(e: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(e, &[seed; 32])
}

/// Stores a minimal ProposalCore with the given quorum so that `quorum_reached`
/// can look it up.
pub(crate) fn store_proposal_with_quorum(e: &Env, proposal_id: &BytesN<32>) {
    let proposer = Address::generate(e);
    let core =
        ProposalCore { proposer, vote_snapshot: 0, vote_end: 0, state: ProposalState::Active };
    e.storage().persistent().set(&ProposalStorageKey::Proposal(proposal_id.clone()), &core);
}

// ################## COUNT VOTE TESTS ##################

#[test]
fn count_vote_for() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 100);
        assert_eq!(counts.against_votes, 0);
        assert_eq!(counts.abstain_votes, 0);
    });
}

#[test]
fn count_vote_against() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_AGAINST, 75);

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.against_votes, 75);
        assert_eq!(counts.for_votes, 0);
        assert_eq!(counts.abstain_votes, 0);
    });
}

#[test]
fn count_vote_abstain() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_ABSTAIN, 50);

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.abstain_votes, 50);
        assert_eq!(counts.for_votes, 0);
        assert_eq!(counts.against_votes, 0);
    });
}

#[test]
fn count_vote_zero_weight() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 0);

        assert!(has_voted(&e, &pid, &alice));
        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 0);
    });
}

// ################## MULTIPLE VOTERS TESTS ##################

#[test]
fn multiple_voters_on_same_proposal() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 60);
        count_vote(&e, &pid, &charlie, VOTE_ABSTAIN, 40);

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 100);
        assert_eq!(counts.against_votes, 60);
        assert_eq!(counts.abstain_votes, 40);

        assert!(has_voted(&e, &pid, &alice));
        assert!(has_voted(&e, &pid, &bob));
        assert!(has_voted(&e, &pid, &charlie));
    });
}

#[test]
fn same_voter_different_proposals() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid1 = proposal_id(&e, 1);
    let pid2 = proposal_id(&e, 2);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid1, &alice, VOTE_FOR, 100);
        count_vote(&e, &pid2, &alice, VOTE_AGAINST, 100);

        let counts1 = get_proposal_vote_counts(&e, &pid1);
        assert_eq!(counts1.for_votes, 100);
        assert_eq!(counts1.against_votes, 0);

        let counts2 = get_proposal_vote_counts(&e, &pid2);
        assert_eq!(counts2.for_votes, 0);
        assert_eq!(counts2.against_votes, 100);
    });
}

#[test]
fn multiple_for_votes_accumulate() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        count_vote(&e, &pid, &bob, VOTE_FOR, 200);
        count_vote(&e, &pid, &charlie, VOTE_FOR, 300);

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 600);
        assert_eq!(counts.against_votes, 0);
        assert_eq!(counts.abstain_votes, 0);
    });
}

// ################## ERROR TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #4216)")]
fn count_vote_fails_on_double_vote() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        count_vote(&e, &pid, &alice, VOTE_FOR, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4216)")]
fn count_vote_fails_on_double_vote_different_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        // Changing vote type on second attempt is still disallowed
        count_vote(&e, &pid, &alice, VOTE_AGAINST, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4217)")]
fn count_vote_fails_on_invalid_vote_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, 3, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4217)")]
fn count_vote_fails_on_large_invalid_vote_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, u32::MAX, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4215)")]
fn count_vote_overflow_for_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, u128::MAX);
        count_vote(&e, &pid, &bob, VOTE_FOR, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4215)")]
fn count_vote_overflow_against_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_AGAINST, u128::MAX);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4215)")]
fn count_vote_overflow_abstain_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_ABSTAIN, u128::MAX);
        count_vote(&e, &pid, &bob, VOTE_ABSTAIN, 1);
    });
}

// ################## EDGE CASES ##################

#[test]
fn large_voting_power() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);
    let large_amount: u128 = u128::MAX / 2;

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, large_amount);

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, large_amount);
    });
}

#[test]
fn proposal_with_only_against_votes_not_succeeded() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_AGAINST, 100);
        assert!(!tally_succeeded(&e, &pid));
    });
}

#[test]
fn proposal_with_only_abstain_votes_not_succeeded() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_ABSTAIN, 100);

        // for (0) is not > against (0)
        assert!(!tally_succeeded(&e, &pid));
    });
}

#[test]
fn full_governance_scenario() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);
    let dave = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 200);
        store_proposal_with_quorum(&e, &pid);

        // Alice votes for with 100 weight
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        assert!(!quorum_reached(&e, &pid, 200)); // 100 < 200

        // Bob votes against with 80 weight
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 80);
        assert!(!quorum_reached(&e, &pid, 200)); // for + abstain = 100 < 200

        // Charlie votes for with 50 weight
        count_vote(&e, &pid, &charlie, VOTE_FOR, 50);
        assert!(!quorum_reached(&e, &pid, 200)); // for + abstain = 150 < 200

        // Dave abstains with 60 weight
        count_vote(&e, &pid, &dave, VOTE_ABSTAIN, 60);
        assert!(quorum_reached(&e, &pid, 200)); // for + abstain = 210 >= 200

        // for (150) > against (80)
        assert!(tally_succeeded(&e, &pid));

        // Verify final tallies
        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 150);
        assert_eq!(counts.against_votes, 80);
        assert_eq!(counts.abstain_votes, 60);

        // Verify all voters are marked
        assert!(has_voted(&e, &pid, &alice));
        assert!(has_voted(&e, &pid, &bob));
        assert!(has_voted(&e, &pid, &charlie));
        assert!(has_voted(&e, &pid, &dave));
    });
}

#[test]
fn defeated_governance_scenario() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        store_proposal_with_quorum(&e, &pid);

        count_vote(&e, &pid, &alice, VOTE_FOR, 50);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 80);
        count_vote(&e, &pid, &charlie, VOTE_ABSTAIN, 60);

        // Quorum: for + abstain = 110 >= 100
        assert!(quorum_reached(&e, &pid, 100));

        // But vote failed: for (50) < against (80)
        assert!(!tally_succeeded(&e, &pid));
    });
}

#[test]
fn independent_proposals_do_not_interfere() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid1 = proposal_id(&e, 1);
    let pid2 = proposal_id(&e, 2);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 50);
        store_proposal_with_quorum(&e, &pid1);
        store_proposal_with_quorum(&e, &pid2);

        // Proposal 1: alice votes for
        count_vote(&e, &pid1, &alice, VOTE_FOR, 100);
        // Proposal 2: alice votes against
        count_vote(&e, &pid2, &alice, VOTE_AGAINST, 100);

        // Proposal 1: bob votes against
        count_vote(&e, &pid1, &bob, VOTE_AGAINST, 200);
        // Proposal 2: bob votes for
        count_vote(&e, &pid2, &bob, VOTE_FOR, 200);

        // Proposal 1: for (100) < against (200) => failed
        assert!(!tally_succeeded(&e, &pid1));
        assert!(quorum_reached(&e, &pid1, 50)); // 100 >= 50

        // Proposal 2: for (200) > against (100) => succeeded
        assert!(tally_succeeded(&e, &pid2));
        assert!(quorum_reached(&e, &pid2, 50)); // 200 >= 50
    });
}

// ################## PROPOSE TESTS ##################

#[test]
fn propose_creates_proposal_successfully() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        let pid =
            propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));

        // Proposal should exist and be in Pending state
        let state = default_state(&e, &pid);
        assert_eq!(state, ProposalState::Pending);

        // Proposer should be recorded
        assert_eq!(get_proposal_proposer(&e, &pid), proposer);
    });
}

#[test]
fn propose_sets_correct_voting_schedule() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let current_ledger = e.ledger().sequence();

    e.as_contract(&contract_address, || {
        let pid =
            propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));

        // voting_delay = 10, voting_period = 100
        let snapshot = get_proposal_snapshot(&e, &pid);
        let deadline = get_proposal_deadline(&e, &pid);
        assert_eq!(snapshot, current_ledger + 10);
        assert_eq!(deadline, current_ledger + 10 + 100);
    });
}

#[test]
fn propose_emits_proposal_created_event() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
    });

    // At least one event should be emitted (ProposalCreated)
    assert!(!e.events().all().events().is_empty());
}

#[test]
#[should_panic(expected = "Error(Contract, #4203)")]
fn propose_fails_with_empty_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let targets: Vec<Address> = vec![&e];
    let functions: Vec<Symbol> = vec![&e];
    let args: Vec<Vec<Val>> = vec![&e];
    let description = String::from_str(&e, "Empty");

    e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4204)")]
fn propose_fails_with_mismatched_lengths() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let targets = vec![&e, Address::generate(&e)];
    let functions = vec![&e, Symbol::new(&e, "a"), Symbol::new(&e, "b")]; // 2 functions
    let args: Vec<Vec<Val>> = vec![&e, vec![&e]]; // 1 args entry
    let description = String::from_str(&e, "Mismatch");

    e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4221)")]
fn propose_fails_with_description_too_long() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, _) = simple_proposal(&e);
    // Create a description that exceeds MAX_DESCRIPTION_LENGTH (8192 bytes)
    let long_desc = String::from_str(&e, &"a".repeat(8193));

    e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, long_desc, &proposer, &stored_settings(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4202)")]
fn propose_fails_with_insufficient_voting_power() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    // threshold is 100, give proposer only 50
    set_mock_voting_power(&e, &token_address, 50);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4201)")]
fn propose_fails_with_duplicate_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description.clone(),
            &proposer,
            &stored_settings(&e),
        );
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
    });
}

#[test]
fn propose_with_exact_threshold() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    // threshold is 100, give exactly 100
    set_mock_voting_power(&e, &token_address, 100);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        let pid =
            propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
        assert_eq!(default_state(&e, &pid), ProposalState::Pending);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4215)")]
fn propose_fails_on_voting_schedule_overflow() {
    let (e, contract_address, token_address) = setup_env_with_token();
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        set_name(&e, String::from_str(&e, "TestGov"));
        set_version(&e, String::from_str(&e, "1.0.0"));
        set_proposal_threshold(&e, 0);
        set_quorum(&e, 100);
        // Use max values to trigger overflow
        set_voting_delay(&e, u32::MAX);
        set_voting_period(&e, 100);

        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e));
    });
}

// ################## CAST_VOTE TESTS ##################

#[test]
fn cast_vote_returns_weight_when_active() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let voter = Address::generate(&e);
        let reason = String::from_str(&e, "");
        let weight = cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));
        assert_eq!(weight, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4205)")]
fn cast_vote_fails_when_pending() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    e.as_contract(&contract_address, || {
        let voter = Address::generate(&e);
        let reason = String::from_str(&e, "");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4205)")]
fn cast_vote_fails_when_defeated() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    let deadline = e.as_contract(&contract_address, || get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    e.as_contract(&contract_address, || {
        let voter = Address::generate(&e);
        let reason = String::from_str(&e, "");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));
    });
}

#[test]
fn cast_vote_records_vote_and_returns_weight() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 500);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Advance to active
    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "I support this");
        let weight = cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));

        assert_eq!(weight, 500);
        assert!(has_voted(&e, &pid, &voter));

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 500);
    });
}

#[test]
fn cast_vote_emits_event() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 100);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    let events_before = e.events().all().events().len();

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "Aye");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));
    });

    // At least one new event (VoteCast)
    assert!(e.events().all().events().len() > events_before);
}

#[test]
#[should_panic(expected = "Error(Contract, #4205)")]
fn cast_vote_fails_when_proposal_not_active() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 500);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Don't advance — still Pending
    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "Early");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4216)")]
fn cast_vote_fails_on_double_vote() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 100);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter, default_state(&e, &pid));
        cast_vote(&e, &pid, VOTE_AGAINST, &reason, &voter, default_state(&e, &pid));
    });
}

#[test]
fn cast_vote_multiple_voters() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 200);

    let proposer = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &alice, default_state(&e, &pid));
        cast_vote(&e, &pid, VOTE_AGAINST, &reason, &bob, default_state(&e, &pid));

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.for_votes, 200);
        assert_eq!(counts.against_votes, 200);
        assert!(has_voted(&e, &pid, &alice));
        assert!(has_voted(&e, &pid, &bob));
    });
}

// ################## CANCEL TESTS ##################

#[test]
fn cancel_pending_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    e.as_contract(&contract_address, || {
        let pid = propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        );
        assert_eq!(default_state(&e, &pid), ProposalState::Pending);

        let cancelled_pid = cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(pid, cancelled_pid);
        assert_eq!(default_state(&e, &pid), ProposalState::Canceled);
    });
}

#[test]
fn cancel_active_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    let pid = e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        )
    });

    // Advance to active
    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Active);
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(default_state(&e, &pid), ProposalState::Canceled);
    });
}

#[test]
fn cancel_defeated_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    let pid = e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        )
    });

    // Advance past deadline to get Defeated
    let deadline = e.as_contract(&contract_address, || get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Defeated);
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(default_state(&e, &pid), ProposalState::Canceled);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4209)")]
fn cancel_fails_when_already_canceled() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        );
        cancel(&e, targets.clone(), functions.clone(), args.clone(), &desc_hash);
        // Second cancel should fail
        cancel(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
fn cancel_fails_for_nonexistent_proposal() {
    let (e, contract_address) = setup_env();
    let desc_hash = BytesN::from_array(&e, &[0u8; 32]);
    let targets: Vec<Address> = vec![&e, Address::generate(&e)];
    let functions: Vec<Symbol> = vec![&e, Symbol::new(&e, "foo")];
    let args: Vec<Vec<Val>> = vec![&e, vec![&e]];

    e.as_contract(&contract_address, || {
        cancel(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
fn cancel_emits_event() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        );
        cancel(&e, targets, functions, args, &desc_hash);
    });

    // At least 2 events: ProposalCreated + ProposalCancelled
    assert!(e.events().all().events().len() >= 2);
}

// ################## EXECUTE TESTS ##################

/// Helper: creates a proposal for a mock target contract, advances to Succeeded
/// state by manually setting it, then returns the proposal ID and desc_hash.
#[contract]
struct TargetContract;

#[contractimpl]
impl TargetContract {
    pub fn do_something(_e: &Env, _val: u32) {}
}

/// Creates a proposal that reaches `Succeeded` through the normal lifecycle:
/// propose → advance to voting → cast a passing vote → advance past voting end.
///
/// Requires `setup_governor_config` (voting_delay=10, voting_period=100,
/// quorum=50) and `set_mock_voting_power` (≥ quorum) to have been called.
fn create_executable_proposal(
    e: &Env,
    contract_address: &Address,
) -> (BytesN<32>, BytesN<32>, Vec<Address>, Vec<Symbol>, Vec<Vec<Val>>) {
    let target = e.register(TargetContract, ());
    let targets = vec![e, target];
    let functions = vec![e, Symbol::new(e, "do_something")];
    let args: Vec<Vec<Val>> = vec![e, vec![e, 42u32.into_val(e)]];
    let description = String::from_str(e, "Executable proposal");
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    let proposer = Address::generate(e);
    let voter = Address::generate(e);

    // Propose (at ledger 100 → vote_start=110, vote_end=210)
    let pid = e.as_contract(contract_address, || {
        propose(
            e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(e),
        )
    });

    // Advance into the voting window and cast a passing vote
    e.ledger().set_sequence_number(111);
    e.as_contract(contract_address, || {
        cast_vote(e, &pid, VOTE_FOR, &String::from_str(e, ""), &voter, default_state(e, &pid));
    });

    // Advance past vote_end so the proposal derives as Succeeded
    e.ledger().set_sequence_number(211);

    (pid, desc_hash, targets, functions, args)
}

#[test]
fn execute_succeeded_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    e.as_contract(&contract_address, || {
        let executed_pid =
            execute(&e, targets, functions, args, &desc_hash, false, default_state(&e, &pid));
        assert_eq!(executed_pid, pid);
        assert_eq!(default_state(&e, &pid), ProposalState::Executed);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4206)")]
fn execute_fails_when_not_succeeded() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    e.as_contract(&contract_address, || {
        let pid = propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        );
        // Proposal is Pending, not Succeeded
        execute(&e, targets, functions, args, &desc_hash, false, default_state(&e, &pid));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4208)")]
fn execute_fails_when_already_executed() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    e.as_contract(&contract_address, || {
        execute(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            &desc_hash,
            false,
            default_state(&e, &pid),
        );
        // Second execution should fail
        execute(&e, targets, functions, args, &desc_hash, false, default_state(&e, &pid));
    });
}

#[test]
fn execute_emits_event() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash, false, default_state(&e, &pid));
    });

    // Verify that a ProposalExecuted event was emitted for our proposal.
    let events = e.events().all();
    let has_executed_event = events.events().iter().any(|event| {
        let soroban_sdk::xdr::ContractEventBody::V0(ref body) = event.body;
        body.topics.iter().any(|t| {
            Symbol::try_from_val(&e, t)
                .map(|s| s == Symbol::new(&e, "proposal_executed"))
                .unwrap_or(false)
        })
    });
    assert!(has_executed_event, "Expected a ProposalExecuted event after execute");
}

#[test]
#[should_panic(expected = "Error(Contract, #4209)")]
fn cancel_fails_when_already_executed() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    e.as_contract(&contract_address, || {
        execute(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            &desc_hash,
            false,
            default_state(&e, &pid),
        );
        // Cancel after execution should fail
        cancel(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
fn execute_fails_for_nonexistent_proposal() {
    let (e, contract_address) = setup_env();
    let desc_hash = BytesN::from_array(&e, &[0u8; 32]);
    let targets: Vec<Address> = vec![&e, Address::generate(&e)];
    let functions: Vec<Symbol> = vec![&e, Symbol::new(&e, "foo")];
    let args: Vec<Vec<Val>> = vec![&e, vec![&e]];

    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash, false, ProposalState::Succeeded);
    });
}

// ################## FULL LIFECYCLE TESTS ##################

#[test]
fn full_proposal_lifecycle_pending_to_active_to_defeated() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Phase 1: Pending
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Pending);
    });

    // Phase 2: Active
    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Active);
    });

    // Phase 3: Defeated (no votes, past deadline)
    let deadline = e.as_contract(&contract_address, || get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Defeated);
    });
}

#[test]
fn full_proposal_lifecycle_to_executed() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    // Should be Succeeded
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Succeeded);
    });

    // Execute
    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash, false, default_state(&e, &pid));
        assert_eq!(default_state(&e, &pid), ProposalState::Executed);
    });
}

#[test]
fn full_proposal_lifecycle_to_canceled() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    let pid = e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        )
    });

    // Cancel while pending
    e.as_contract(&contract_address, || {
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(default_state(&e, &pid), ProposalState::Canceled);
    });
}

// ################## QUEUE TESTS ##################

#[test]
fn queue_succeeded_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    // Verify it's Succeeded first
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Succeeded);
    });

    // Queue with queue_enabled = true
    e.as_contract(&contract_address, || {
        let queued_pid =
            queue(&e, targets, functions, args, &desc_hash, 500, default_state(&e, &pid));
        assert_eq!(queued_pid, pid);
        assert_eq!(default_state(&e, &pid), ProposalState::Queued);
    });
}

// NOTE: The `QueueNotEnabled` check is enforced by the `Governor` trait's
// default `queue` implementation (not by `queue`), so it is tested
// at the integration level in the example contracts.

#[test]
#[should_panic(expected = "Error(Contract, #4206)")]
fn queue_fails_when_not_succeeded() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_bytes()).to_bytes();

    // Create proposal (stays Pending)
    let pid = e.as_contract(&contract_address, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        )
    });

    // Trying to queue a Pending proposal should fail with ProposalNotSuccessful
    e.as_contract(&contract_address, || {
        queue(&e, targets, functions, args, &desc_hash, 500, default_state(&e, &pid));
    });
}

#[test]
fn full_proposal_lifecycle_with_queue() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    // Queue
    e.as_contract(&contract_address, || {
        queue(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            &desc_hash,
            500,
            default_state(&e, &pid),
        );
        assert_eq!(default_state(&e, &pid), ProposalState::Queued);
    });

    // Execute with queue_enabled = true (requires Queued state)
    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash, true, default_state(&e, &pid));
        assert_eq!(default_state(&e, &pid), ProposalState::Executed);
    });
}

#[test]
fn cancel_queued_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    // Queue
    e.as_contract(&contract_address, || {
        queue(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            &desc_hash,
            500,
            default_state(&e, &pid),
        );
        assert_eq!(default_state(&e, &pid), ProposalState::Queued);
    });

    // Cancel a queued proposal should work
    e.as_contract(&contract_address, || {
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(default_state(&e, &pid), ProposalState::Canceled);
    });
}

// ################## CONFIGURATION OVERRIDE TESTS ##################

const OVERRIDDEN_THRESHOLD: u128 = 500;
const OVERRIDDEN_VOTING_DELAY: u32 = 50;
const OVERRIDDEN_VOTING_PERIOD: u32 = 300;

/// A governor that overrides the proposal threshold, voting delay and voting
/// period with values different from the stored ones.
#[contract]
struct OverrideGovernor;

#[contractimpl(contracttrait)]
impl GovernorSettings for OverrideGovernor {
    fn proposal_threshold(_e: &Env) -> u128 {
        OVERRIDDEN_THRESHOLD
    }

    fn voting_delay(_e: &Env) -> u32 {
        OVERRIDDEN_VOTING_DELAY
    }

    fn voting_period(_e: &Env) -> u32 {
        OVERRIDDEN_VOTING_PERIOD
    }
}

#[contractimpl(contracttrait)]
impl GovernorQueries for OverrideGovernor {}

#[contractimpl(contracttrait)]
impl Governor for OverrideGovernor {
    fn execute(
        _e: &Env,
        _targets: Vec<Address>,
        _functions: Vec<Symbol>,
        _args: Vec<Vec<Val>>,
        _description_hash: BytesN<32>,
        _executor: Address,
    ) -> BytesN<32> {
        unimplemented!("not used in these tests")
    }

    fn cancel(
        _e: &Env,
        _targets: Vec<Address>,
        _functions: Vec<Symbol>,
        _args: Vec<Vec<Val>>,
        _description_hash: BytesN<32>,
        _operator: Address,
    ) -> BytesN<32> {
        unimplemented!("not used in these tests")
    }
}

/// Registers an `OverrideGovernor` wired to a mock token that reports
/// `voting_power` for every account. When `store_config` is true, the stored
/// configuration (threshold 100, delay 10, period 100) differs from the
/// overrides.
fn setup_override_governor(
    voting_power: u128,
    store_config: bool,
) -> (Env, OverrideGovernorClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set_sequence_number(100);
    let governor_address = e.register(OverrideGovernor, ());
    let token_address = e.register(MockTokenContract, ());
    e.as_contract(&governor_address, || {
        set_token_contract(&e, &token_address);
    });
    if store_config {
        setup_governor_config(&e, &governor_address);
    }
    set_mock_voting_power(&e, &token_address, voting_power);
    let client = OverrideGovernorClient::new(&e, &governor_address);
    (e, client)
}

#[test]
fn propose_enforces_overridden_voting_schedule() {
    let (e, governor) = setup_override_governor(1000, true);
    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = governor.propose(&targets, &functions, &args, &description, &proposer);

    let snapshot = governor.proposal_snapshot(&pid);
    assert_eq!(snapshot, 100 + OVERRIDDEN_VOTING_DELAY);
    assert_eq!(governor.proposal_deadline(&pid), snapshot + OVERRIDDEN_VOTING_PERIOD);
}

#[test]
#[should_panic(expected = "Error(Contract, #4202)")]
fn propose_enforces_overridden_threshold() {
    // 200 clears the stored threshold (100) but not the overridden one (500).
    let (e, governor) = setup_override_governor(200, true);
    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    governor.propose(&targets, &functions, &args, &description, &proposer);
}

#[test]
fn propose_with_overrides_needs_no_stored_config() {
    let (e, governor) = setup_override_governor(OVERRIDDEN_THRESHOLD, false);
    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = governor.propose(&targets, &functions, &args, &description, &proposer);

    assert_eq!(governor.proposal_proposer(&pid), proposer);
    assert_eq!(governor.proposal_snapshot(&pid), 100 + OVERRIDDEN_VOTING_DELAY);
}

// ################## OUTCOME OVERRIDE TESTS ##################

/// At least two thirds of the `for` and `against` votes must be `for`.
fn two_thirds_in_favor(e: &Env, proposal_id: &BytesN<32>) -> bool {
    let counts = get_proposal_vote_counts(e, proposal_id);
    counts.for_votes * 3 >= (counts.for_votes + counts.against_votes) * 2
}

/// Executes through the storage helper, passing the state reported by
/// `GovernorQueries::proposal_state`, as the `Governor::execute` docs advise.
fn execute_with_reported_state<G: Governor>(
    e: &Env,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    description_hash: BytesN<32>,
) -> BytesN<32> {
    let proposal_id = hash_proposal(e, &targets, &functions, &args, &description_hash);
    let state = G::proposal_state(e, proposal_id);
    execute(e, targets, functions, args, &description_hash, G::proposals_need_queuing(e), state)
}

/// A governor with a two-thirds rule set through the `proposal_succeeded`
/// hook, with queueing enabled.
#[contract]
struct SucceededHookGovernor;

#[contractimpl(contracttrait)]
impl GovernorSettings for SucceededHookGovernor {
    fn proposals_need_queuing(_e: &Env) -> bool {
        true
    }
}

#[contractimpl(contracttrait)]
impl GovernorQueries for SucceededHookGovernor {
    fn proposal_succeeded(e: &Env, proposal_id: BytesN<32>) -> bool {
        let quorum = Self::quorum(e, get_proposal_snapshot(e, &proposal_id));
        quorum_reached(e, &proposal_id, quorum) && two_thirds_in_favor(e, &proposal_id)
    }
}

#[contractimpl(contracttrait)]
impl Governor for SucceededHookGovernor {
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        _executor: Address,
    ) -> BytesN<32> {
        execute_with_reported_state::<Self>(e, targets, functions, args, description_hash)
    }

    fn cancel(
        _e: &Env,
        _targets: Vec<Address>,
        _functions: Vec<Symbol>,
        _args: Vec<Vec<Val>>,
        _description_hash: BytesN<32>,
        _operator: Address,
    ) -> BytesN<32> {
        unimplemented!("not used in these tests")
    }
}

/// A governor with the same two-thirds rule set by overriding
/// `proposal_state`, with queueing disabled.
#[contract]
struct StateOverrideGovernor;

#[contractimpl(contracttrait)]
impl GovernorSettings for StateOverrideGovernor {}

#[contractimpl(contracttrait)]
impl GovernorQueries for StateOverrideGovernor {
    fn proposal_state(e: &Env, proposal_id: BytesN<32>) -> ProposalState {
        let quorum = Self::quorum(e, get_proposal_snapshot(e, &proposal_id));
        let succeeded =
            quorum_reached(e, &proposal_id, quorum) && two_thirds_in_favor(e, &proposal_id);
        get_proposal_state(e, &proposal_id, succeeded)
    }
}

#[contractimpl(contracttrait)]
impl Governor for StateOverrideGovernor {
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        _executor: Address,
    ) -> BytesN<32> {
        execute_with_reported_state::<Self>(e, targets, functions, args, description_hash)
    }

    fn cancel(
        _e: &Env,
        _targets: Vec<Address>,
        _functions: Vec<Symbol>,
        _args: Vec<Vec<Val>>,
        _description_hash: BytesN<32>,
        _operator: Address,
    ) -> BytesN<32> {
        unimplemented!("not used in these tests")
    }
}

struct OutcomeSetup {
    e: Env,
    governor: Address,
    pid: BytesN<32>,
    targets: Vec<Address>,
    functions: Vec<Symbol>,
    args: Vec<Vec<Val>>,
    desc_hash: BytesN<32>,
}

/// Registers the governor returned by `register`, creates an executable
/// proposal, records `for_votes` and `against_votes`, and ends the voting
/// period. The quorum is 50.
fn setup_outcome(
    register: impl FnOnce(&Env) -> Address,
    for_votes: u128,
    against_votes: u128,
) -> OutcomeSetup {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set_sequence_number(100);
    let governor = register(&e);
    let token_address = e.register(MockTokenContract, ());
    e.as_contract(&governor, || set_token_contract(&e, &token_address));
    setup_governor_config(&e, &governor);
    set_mock_voting_power(&e, &token_address, 1000);

    let target = e.register(TargetContract, ());
    let targets = vec![&e, target];
    let functions = vec![&e, Symbol::new(&e, "do_something")];
    let args: Vec<Vec<Val>> = vec![&e, vec![&e, 42u32.into_val(&e)]];
    let description = String::from_str(&e, "Two-thirds proposal");
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

    let proposer = Address::generate(&e);
    let pid = e.as_contract(&governor, || {
        propose(
            &e,
            targets.clone(),
            functions.clone(),
            args.clone(),
            description,
            &proposer,
            &stored_settings(&e),
        )
    });

    let snapshot = e.as_contract(&governor, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);
    e.as_contract(&governor, || {
        count_vote(&e, &pid, &Address::generate(&e), VOTE_FOR, for_votes);
        count_vote(&e, &pid, &Address::generate(&e), VOTE_AGAINST, against_votes);
    });
    let deadline = e.as_contract(&governor, || get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    OutcomeSetup { e, governor, pid, targets, functions, args, desc_hash }
}

#[test]
#[should_panic(expected = "Error(Contract, #4206)")]
fn succeeded_hook_defeated_proposal_cannot_be_queued() {
    // 60 / 100 is a simple majority, but below two thirds.
    let s = setup_outcome(|e| e.register(SucceededHookGovernor, ()), 60, 40);
    let governor = SucceededHookGovernorClient::new(&s.e, &s.governor);

    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Defeated);
    let operator = Address::generate(&s.e);
    governor.queue(&s.targets, &s.functions, &s.args, &s.desc_hash, &500, &operator);
}

#[test]
fn succeeded_hook_succeeded_proposal_is_queued_and_executed() {
    let s = setup_outcome(|e| e.register(SucceededHookGovernor, ()), 70, 30);
    let governor = SucceededHookGovernorClient::new(&s.e, &s.governor);
    let operator = Address::generate(&s.e);

    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Succeeded);
    governor.queue(&s.targets, &s.functions, &s.args, &s.desc_hash, &500, &operator);
    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Queued);
    governor.execute(&s.targets, &s.functions, &s.args, &s.desc_hash, &operator);
    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #4206)")]
fn proposal_state_override_defeated_proposal_cannot_be_executed() {
    let s = setup_outcome(|e| e.register(StateOverrideGovernor, ()), 60, 40);
    let governor = StateOverrideGovernorClient::new(&s.e, &s.governor);

    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Defeated);
    let executor = Address::generate(&s.e);
    governor.execute(&s.targets, &s.functions, &s.args, &s.desc_hash, &executor);
}

#[test]
fn proposal_state_override_succeeded_proposal_is_executed() {
    let s = setup_outcome(|e| e.register(StateOverrideGovernor, ()), 70, 30);
    let governor = StateOverrideGovernorClient::new(&s.e, &s.governor);

    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Succeeded);
    let executor = Address::generate(&s.e);
    governor.execute(&s.targets, &s.functions, &s.args, &s.desc_hash, &executor);
    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #4205)")]
fn cast_vote_follows_reported_state() {
    // Voting has ended, so the reported state is `Defeated`, not `Active`.
    let s = setup_outcome(|e| e.register(StateOverrideGovernor, ()), 60, 40);
    let governor = StateOverrideGovernorClient::new(&s.e, &s.governor);
    let voter = Address::generate(&s.e);
    governor.cast_vote(&s.pid, &VOTE_FOR, &String::from_str(&s.e, ""), &voter);
}

/// A governor that counts every vote type (including `against`) toward the
/// quorum, by overriding `proposal_succeeded`.
#[contract]
struct AllVotesQuorumGovernor;

#[contractimpl(contracttrait)]
impl GovernorSettings for AllVotesQuorumGovernor {}

#[contractimpl(contracttrait)]
impl GovernorQueries for AllVotesQuorumGovernor {
    fn proposal_succeeded(e: &Env, proposal_id: BytesN<32>) -> bool {
        let quorum = Self::quorum(e, get_proposal_snapshot(e, &proposal_id));
        let counts = get_proposal_vote_counts(e, &proposal_id);
        let participation = counts.for_votes + counts.against_votes + counts.abstain_votes;
        participation >= quorum && tally_succeeded(e, &proposal_id)
    }
}

#[contractimpl(contracttrait)]
impl Governor for AllVotesQuorumGovernor {
    fn execute(
        e: &Env,
        targets: Vec<Address>,
        functions: Vec<Symbol>,
        args: Vec<Vec<Val>>,
        description_hash: BytesN<32>,
        _executor: Address,
    ) -> BytesN<32> {
        execute_with_reported_state::<Self>(e, targets, functions, args, description_hash)
    }

    fn cancel(
        _e: &Env,
        _targets: Vec<Address>,
        _functions: Vec<Symbol>,
        _args: Vec<Vec<Val>>,
        _description_hash: BytesN<32>,
        _operator: Address,
    ) -> BytesN<32> {
        unimplemented!("not used in these tests")
    }
}

#[test]
fn succeeded_hook_can_count_against_votes_toward_quorum() {
    // `for` alone (30) misses the quorum of 50, but `for` + `against` (55)
    // reaches it, and `for` still exceeds `against`.
    let s = setup_outcome(|e| e.register(AllVotesQuorumGovernor, ()), 30, 25);
    let governor = AllVotesQuorumGovernorClient::new(&s.e, &s.governor);

    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Succeeded);
    let executor = Address::generate(&s.e);
    governor.execute(&s.targets, &s.functions, &s.args, &s.desc_hash, &executor);
    assert_eq!(governor.proposal_state(&s.pid), ProposalState::Executed);

    // The same votes are `Defeated` under the default rule.
    s.e.as_contract(&s.governor, || {
        let quorum = get_quorum(&s.e, get_proposal_snapshot(&s.e, &s.pid));
        assert!(!quorum_reached(&s.e, &s.pid, quorum));
    });
}
