use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events, Ledger},
    vec,
    xdr::ToXdr,
    Address, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};

use crate::governor::{
    storage::{
        self, cancel, cast_vote, count_vote, counting_mode, execute, get_proposal_vote_counts,
        get_quorum, get_token_contract, has_voted, hash_proposal, propose, quorum_reached,
        set_quorum, set_token_contract, tally_succeeded,
    },
    ProposalState, VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR,
};

#[contract]
struct MockContract;

/// A mock token contract that implements `get_votes_at_checkpoint`.
/// The voting power to return is stored under the `"power"` key.
#[contract]
struct MockTokenContract;

#[contractimpl]
impl MockTokenContract {
    pub fn get_votes_at_checkpoint(e: &Env, _account: Address, _ledger: u32) -> u128 {
        e.storage().instance().get(&Symbol::new(e, "power")).unwrap_or(0)
    }
}

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
}

/// Sets up a governor contract with a mock token contract.
/// Returns (env, governor_address, token_address).
fn setup_env_with_token() -> (Env, Address, Address) {
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

/// Sets the voting power the mock token contract will return.
fn set_mock_voting_power(e: &Env, token_address: &Address, power: u128) {
    e.as_contract(token_address, || {
        e.storage().instance().set(&Symbol::new(e, "power"), &power);
    });
}

/// Initializes a governor with standard config for proposal tests.
fn setup_governor_config(e: &Env, contract_address: &Address) {
    e.as_contract(contract_address, || {
        storage::set_name(e, String::from_str(e, "TestGov"));
        storage::set_version(e, String::from_str(e, "1.0.0"));
        storage::set_proposal_threshold(e, 100);
        storage::set_voting_delay(e, 10);
        storage::set_voting_period(e, 100);
        set_quorum(e, 50);
    });
}

/// Creates a simple single-action proposal parameter set.
fn simple_proposal(e: &Env) -> (Vec<Address>, Vec<Symbol>, Vec<Vec<Val>>, String) {
    let target = Address::generate(e);
    let targets = vec![e, target];
    let functions = vec![e, Symbol::new(e, "do_something")];
    let args: Vec<Vec<Val>> = vec![e, vec![e, 42u32.into_val(e)]];
    let description = String::from_str(e, "Test proposal");
    (targets, functions, args, description)
}

fn proposal_id(e: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(e, &[seed; 32])
}

// ################## INITIAL STATE TESTS ##################

#[test]
fn initial_state_has_no_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        assert!(!has_voted(&e, &pid, &alice));

        let counts = get_proposal_vote_counts(&e, &pid);
        assert_eq!(counts.against_votes, 0);
        assert_eq!(counts.for_votes, 0);
        assert_eq!(counts.abstain_votes, 0);
    });
}

#[test]
fn initial_vote_not_succeeded_and_no_votes() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        // With no votes, for_votes (0) is not > against_votes (0)
        assert!(!tally_succeeded(&e, &pid));
    });
}

// ################## COUNTING MODE TESTS ##################

#[test]
fn counting_mode_returns_simple() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        let mode = counting_mode(&e);
        assert_eq!(mode, soroban_sdk::Symbol::new(&e, "simple"));
    });
}

// ################## QUORUM MANAGEMENT TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #5018)")]
fn get_quorum_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_quorum(&e);
    });
}

#[test]
fn set_and_get_quorum() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 1000);
        assert_eq!(get_quorum(&e), 1000);
    });
}

#[test]
fn update_quorum() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 1000);
        assert_eq!(get_quorum(&e), 1000);

        set_quorum(&e, 2000);
        assert_eq!(get_quorum(&e), 2000);
    });
}

#[test]
fn set_quorum_emits_event() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 500);
    });

    assert_eq!(e.events().all().events().len(), 1);
}

#[test]
fn update_quorum_emits_event_with_old_value() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 500);
        set_quorum(&e, 1000);
    });

    assert_eq!(e.events().all().events().len(), 2);
}

#[test]
fn set_quorum_to_zero() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 0);
        assert_eq!(get_quorum(&e), 0);
    });
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

// ################## HAS_VOTED TESTS ##################

#[test]
fn has_voted_returns_false_before_voting() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        assert!(!has_voted(&e, &pid, &alice));
    });
}

#[test]
fn has_voted_returns_true_after_voting() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        assert!(has_voted(&e, &pid, &alice));
    });
}

#[test]
fn has_voted_is_per_proposal() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid1 = proposal_id(&e, 1);
    let pid2 = proposal_id(&e, 2);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid1, &alice, VOTE_FOR, 100);

        assert!(has_voted(&e, &pid1, &alice));
        assert!(!has_voted(&e, &pid2, &alice));
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

// ################## TALLY_SUCCEEDED TESTS ##################

#[test]
fn tally_succeeded_when_for_exceeds_against() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 50);

        assert!(tally_succeeded(&e, &pid));
    });
}

#[test]
fn vote_not_succeeded_when_against_exceeds_for() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 50);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 100);

        assert!(!tally_succeeded(&e, &pid));
    });
}

#[test]
fn vote_not_succeeded_when_tied() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 100);

        // Tied: for is not strictly greater than against
        assert!(!tally_succeeded(&e, &pid));
    });
}

#[test]
fn tally_succeeded_ignores_abstain() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, VOTE_FOR, 1);
        count_vote(&e, &pid, &bob, VOTE_ABSTAIN, 1000);

        // for (1) > against (0), abstain does not count against success
        assert!(tally_succeeded(&e, &pid));
    });
}

// ################## QUORUM_REACHED TESTS ##################

#[test]
fn quorum_reached_with_for_votes_only() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);

        assert!(quorum_reached(&e, &pid));
    });
}

#[test]
fn quorum_reached_with_abstain_votes_only() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        count_vote(&e, &pid, &alice, VOTE_ABSTAIN, 100);

        assert!(quorum_reached(&e, &pid));
    });
}

#[test]
fn quorum_reached_with_for_and_abstain_combined() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        count_vote(&e, &pid, &alice, VOTE_FOR, 60);
        count_vote(&e, &pid, &bob, VOTE_ABSTAIN, 40);

        // 60 + 40 = 100 >= 100
        assert!(quorum_reached(&e, &pid));
    });
}

#[test]
fn quorum_not_reached_when_insufficient() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        count_vote(&e, &pid, &alice, VOTE_FOR, 99);

        assert!(!quorum_reached(&e, &pid));
    });
}

#[test]
fn quorum_ignores_against_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        count_vote(&e, &pid, &alice, VOTE_AGAINST, 200);
        count_vote(&e, &pid, &bob, VOTE_FOR, 50);

        // Only for + abstain count toward quorum: 50 < 100
        assert!(!quorum_reached(&e, &pid));
    });
}

#[test]
fn quorum_reached_exactly_at_threshold() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 100);
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);

        // Exactly at threshold: 100 >= 100
        assert!(quorum_reached(&e, &pid));
    });
}

#[test]
fn quorum_zero_always_reached() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        set_quorum(&e, 0);

        // 0 >= 0 with no votes
        assert!(quorum_reached(&e, &pid));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5018)")]
fn quorum_reached_fails_when_quorum_not_set() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        quorum_reached(&e, &pid);
    });
}

// ################## ERROR TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #5016)")]
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
#[should_panic(expected = "Error(Contract, #5016)")]
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
#[should_panic(expected = "Error(Contract, #5017)")]
fn count_vote_fails_on_invalid_vote_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, 3, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5017)")]
fn count_vote_fails_on_large_invalid_vote_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, u32::MAX, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5015)")]
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
#[should_panic(expected = "Error(Contract, #5015)")]
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
#[should_panic(expected = "Error(Contract, #5015)")]
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

        // Alice votes for with 100 weight
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);
        assert!(!quorum_reached(&e, &pid)); // 100 < 200

        // Bob votes against with 80 weight
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 80);
        assert!(!quorum_reached(&e, &pid)); // for + abstain = 100 < 200

        // Charlie votes for with 50 weight
        count_vote(&e, &pid, &charlie, VOTE_FOR, 50);
        assert!(!quorum_reached(&e, &pid)); // for + abstain = 150 < 200

        // Dave abstains with 60 weight
        count_vote(&e, &pid, &dave, VOTE_ABSTAIN, 60);
        assert!(quorum_reached(&e, &pid)); // for + abstain = 210 >= 200

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

        count_vote(&e, &pid, &alice, VOTE_FOR, 50);
        count_vote(&e, &pid, &bob, VOTE_AGAINST, 80);
        count_vote(&e, &pid, &charlie, VOTE_ABSTAIN, 60);

        // Quorum: for + abstain = 110 >= 100
        assert!(quorum_reached(&e, &pid));

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
        assert!(quorum_reached(&e, &pid1)); // 100 >= 50

        // Proposal 2: for (200) > against (100) => succeeded
        assert!(tally_succeeded(&e, &pid2));
        assert!(quorum_reached(&e, &pid2)); // 200 >= 50
    });
}

// ################## CONFIG GETTER/SETTER TESTS ##################

#[test]
fn set_and_get_name() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::set_name(&e, String::from_str(&e, "MyGovernor"));
        assert_eq!(storage::get_name(&e), String::from_str(&e, "MyGovernor"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5013)")]
fn get_name_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::get_name(&e);
    });
}

#[test]
fn set_and_get_version() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::set_version(&e, String::from_str(&e, "1.0.0"));
        assert_eq!(storage::get_version(&e), String::from_str(&e, "1.0.0"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5014)")]
fn get_version_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::get_version(&e);
    });
}

#[test]
fn set_and_get_proposal_threshold() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::set_proposal_threshold(&e, 500);
        assert_eq!(storage::get_proposal_threshold(&e), 500);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5012)")]
fn get_proposal_threshold_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::get_proposal_threshold(&e);
    });
}

#[test]
fn set_and_get_voting_delay() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::set_voting_delay(&e, 10);
        assert_eq!(storage::get_voting_delay(&e), 10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5010)")]
fn get_voting_delay_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::get_voting_delay(&e);
    });
}

#[test]
fn set_and_get_voting_period() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::set_voting_period(&e, 100);
        assert_eq!(storage::get_voting_period(&e), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5011)")]
fn get_voting_period_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::get_voting_period(&e);
    });
}

#[test]
fn update_config_values() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        storage::set_voting_delay(&e, 10);
        storage::set_voting_period(&e, 100);
        storage::set_proposal_threshold(&e, 500);

        storage::set_voting_delay(&e, 20);
        storage::set_voting_period(&e, 200);
        storage::set_proposal_threshold(&e, 1000);

        assert_eq!(storage::get_voting_delay(&e), 20);
        assert_eq!(storage::get_voting_period(&e), 200);
        assert_eq!(storage::get_proposal_threshold(&e), 1000);
    });
}

// ################## TOKEN CONTRACT TESTS ##################

#[test]
fn set_and_get_token_contract() {
    let (e, contract_address) = setup_env();
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_token_contract(&e, &token);
        assert_eq!(get_token_contract(&e), token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5020)")]
fn get_token_contract_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_token_contract(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5019)")]
fn set_token_contract_fails_when_already_set() {
    let (e, contract_address) = setup_env();
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_token_contract(&e, &token1);
        set_token_contract(&e, &token2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5019)")]
fn set_token_contract_fails_on_same_address() {
    let (e, contract_address) = setup_env();
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_token_contract(&e, &token);
        set_token_contract(&e, &token);
    });
}

// ################## HASH PROPOSAL TESTS ##################

#[test]
fn hash_proposal_is_deterministic() {
    let (e, _) = setup_env();
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    let hash1 = hash_proposal(&e, &targets, &functions, &args, &desc_hash);
    let hash2 = hash_proposal(&e, &targets, &functions, &args, &desc_hash);

    assert_eq!(hash1, hash2);
}

#[test]
fn hash_proposal_differs_with_different_description() {
    let (e, _) = setup_env();
    let (targets, functions, args, _) = simple_proposal(&e);

    let desc1 = String::from_str(&e, "Proposal A");
    let desc2 = String::from_str(&e, "Proposal B");
    let hash1_bytes = e.crypto().keccak256(&desc1.clone().to_xdr(&e)).to_bytes();
    let hash2_bytes = e.crypto().keccak256(&desc2.clone().to_xdr(&e)).to_bytes();

    let id1 = hash_proposal(&e, &targets, &functions, &args, &hash1_bytes);
    let id2 = hash_proposal(&e, &targets, &functions, &args, &hash2_bytes);

    assert_ne!(id1, id2);
}

#[test]
fn hash_proposal_differs_with_different_targets() {
    let (e, _) = setup_env();
    let description = String::from_str(&e, "Test");
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();
    let functions = vec![&e, Symbol::new(&e, "do_something")];
    let args: Vec<Vec<Val>> = vec![&e, vec![&e, 1u32.into_val(&e)]];

    let targets1 = vec![&e, Address::generate(&e)];
    let targets2 = vec![&e, Address::generate(&e)];

    let id1 = hash_proposal(&e, &targets1, &functions, &args, &desc_hash);
    let id2 = hash_proposal(&e, &targets2, &functions, &args, &desc_hash);

    assert_ne!(id1, id2);
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
        let pid = propose(&e, targets, functions, args, description, &proposer);

        // Proposal should exist and be in Pending state
        let state = storage::get_proposal_state(&e, &pid);
        assert_eq!(state, ProposalState::Pending);

        // Proposer should be recorded
        assert_eq!(storage::get_proposal_proposer(&e, &pid), proposer);
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
        let pid = propose(&e, targets, functions, args, description, &proposer);

        // voting_delay = 10, voting_period = 100
        let snapshot = storage::get_proposal_snapshot(&e, &pid);
        let deadline = storage::get_proposal_deadline(&e, &pid);
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
        propose(&e, targets, functions, args, description, &proposer);
    });

    // At least one event should be emitted (ProposalCreated)
    assert!(!e.events().all().events().is_empty());
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
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
        propose(&e, targets, functions, args, description, &proposer);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5004)")]
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
        propose(&e, targets, functions, args, description, &proposer);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5002)")]
fn propose_fails_with_insufficient_voting_power() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    // threshold is 100, give proposer only 50
    set_mock_voting_power(&e, &token_address, 50);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5001)")]
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
        );
        propose(&e, targets, functions, args, description, &proposer);
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
        let pid = propose(&e, targets, functions, args, description, &proposer);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Pending);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5015)")]
fn propose_fails_on_voting_schedule_overflow() {
    let (e, contract_address, token_address) = setup_env_with_token();
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    e.as_contract(&contract_address, || {
        storage::set_name(&e, String::from_str(&e, "TestGov"));
        storage::set_version(&e, String::from_str(&e, "1.0.0"));
        storage::set_proposal_threshold(&e, 0);
        // Use max values to trigger overflow
        storage::set_voting_delay(&e, u32::MAX);
        storage::set_voting_period(&e, 100);

        propose(&e, targets, functions, args, description, &proposer);
    });
}

// ################## PROPOSAL QUERY TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn get_proposal_state_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        storage::get_proposal_state(&e, &pid);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn get_proposal_snapshot_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        storage::get_proposal_snapshot(&e, &pid);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn get_proposal_deadline_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        storage::get_proposal_deadline(&e, &pid);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn get_proposal_proposer_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        storage::get_proposal_proposer(&e, &pid);
    });
}

// ################## PROPOSAL STATE TRANSITION TESTS ##################

#[test]
fn proposal_transitions_to_active() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    // Advance past voting delay (vote_start). Voting opens after snapshot.
    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Active);
    });
}

#[test]
fn proposal_transitions_to_defeated_after_voting_period() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    // Advance past deadline
    let deadline = e.as_contract(&contract_address, || storage::get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Defeated);
    });
}

#[test]
fn proposal_pending_before_voting_starts() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    // Still within voting delay
    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Pending);
    });
}

#[test]
fn check_proposal_state_returns_snapshot_when_active() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let returned_snapshot = storage::check_proposal_state(&e, &pid);
        assert_eq!(returned_snapshot, snapshot);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5005)")]
fn check_proposal_state_fails_when_pending() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    e.as_contract(&contract_address, || {
        storage::check_proposal_state(&e, &pid);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5005)")]
fn check_proposal_state_fails_when_defeated() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    let deadline = e.as_contract(&contract_address, || storage::get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    e.as_contract(&contract_address, || {
        storage::check_proposal_state(&e, &pid);
    });
}

// ################## CAST_VOTE TESTS ##################

#[test]
fn cast_vote_records_vote_and_returns_weight() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 500);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    // Advance to active
    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "I support this");
        let weight = cast_vote(&e, &pid, VOTE_FOR, &reason, &voter);

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
        propose(&e, targets, functions, args, description, &proposer)
    });

    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    let events_before = e.events().all().events().len();

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "Aye");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter);
    });

    // At least one new event (VoteCast)
    assert!(e.events().all().events().len() > events_before);
}

#[test]
#[should_panic(expected = "Error(Contract, #5005)")]
fn cast_vote_fails_when_proposal_not_active() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 500);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    // Don't advance — still Pending
    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "Early");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5016)")]
fn cast_vote_fails_on_double_vote() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 100);

    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer)
    });

    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &voter);
        cast_vote(&e, &pid, VOTE_AGAINST, &reason, &voter);
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
        propose(&e, targets, functions, args, description, &proposer)
    });

    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        let reason = String::from_str(&e, "");
        cast_vote(&e, &pid, VOTE_FOR, &reason, &alice);
        cast_vote(&e, &pid, VOTE_AGAINST, &reason, &bob);

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
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    e.as_contract(&contract_address, || {
        let pid =
            propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Pending);

        let cancelled_pid = cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(pid, cancelled_pid);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Canceled);
    });
}

#[test]
fn cancel_active_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer)
    });

    // Advance to active
    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Active);
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Canceled);
    });
}

#[test]
fn cancel_defeated_proposal() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer)
    });

    // Advance past deadline to get Defeated
    let deadline = e.as_contract(&contract_address, || storage::get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Defeated);
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Canceled);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5009)")]
fn cancel_fails_when_already_canceled() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    e.as_contract(&contract_address, || {
        propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer);
        cancel(&e, targets.clone(), functions.clone(), args.clone(), &desc_hash);
        // Second cancel should fail
        cancel(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
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
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    e.as_contract(&contract_address, || {
        propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer);
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

fn create_executable_proposal(
    e: &Env,
    contract_address: &Address,
) -> (BytesN<32>, BytesN<32>, Vec<Address>, Vec<Symbol>, Vec<Vec<Val>>) {
    let target = e.register(TargetContract, ());
    let targets = vec![e, target];
    let functions = vec![e, Symbol::new(e, "do_something")];
    let args: Vec<Vec<Val>> = vec![e, vec![e, 42u32.into_val(e)]];
    let description = String::from_str(e, "Executable proposal");
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(e)).to_bytes();

    let proposer = Address::generate(e);

    let pid = e.as_contract(contract_address, || {
        propose(e, targets.clone(), functions.clone(), args.clone(), description, &proposer)
    });

    // Manually set to Succeeded by writing directly to storage
    e.as_contract(contract_address, || {
        let core = storage::get_proposal_core(e, &pid);
        let succeeded_core = storage::ProposalCore { state: ProposalState::Succeeded, ..core };
        e.storage()
            .persistent()
            .set(&storage::GovernorStorageKey::Proposal(pid.clone()), &succeeded_core);
    });

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
        let executed_pid = execute(&e, targets, functions, args, &desc_hash);
        assert_eq!(executed_pid, pid);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Executed);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5006)")]
fn execute_fails_when_not_succeeded() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    e.as_contract(&contract_address, || {
        propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer);
        // Proposal is Pending, not Succeeded
        execute(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5008)")]
fn execute_fails_when_already_executed() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (_pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    e.as_contract(&contract_address, || {
        execute(&e, targets.clone(), functions.clone(), args.clone(), &desc_hash);
        // Second execution should fail
        execute(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
fn execute_emits_event() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (_pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    let events_before = e.events().all().events().len();

    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash);
    });

    assert!(e.events().all().events().len() > events_before);
}

#[test]
#[should_panic(expected = "Error(Contract, #5009)")]
fn cancel_fails_when_already_executed() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let (_pid, desc_hash, targets, functions, args) =
        create_executable_proposal(&e, &contract_address);

    e.as_contract(&contract_address, || {
        execute(&e, targets.clone(), functions.clone(), args.clone(), &desc_hash);
        // Cancel after execution should fail
        cancel(&e, targets, functions, args, &desc_hash);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn execute_fails_for_nonexistent_proposal() {
    let (e, contract_address) = setup_env();
    let desc_hash = BytesN::from_array(&e, &[0u8; 32]);
    let targets: Vec<Address> = vec![&e, Address::generate(&e)];
    let functions: Vec<Symbol> = vec![&e, Symbol::new(&e, "foo")];
    let args: Vec<Vec<Val>> = vec![&e, vec![&e]];

    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash);
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
        propose(&e, targets, functions, args, description, &proposer)
    });

    // Phase 1: Pending
    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Pending);
    });

    // Phase 2: Active
    let snapshot = e.as_contract(&contract_address, || storage::get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);
    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Active);
    });

    // Phase 3: Defeated (no votes, past deadline)
    let deadline = e.as_contract(&contract_address, || storage::get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);
    e.as_contract(&contract_address, || {
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Defeated);
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
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Succeeded);
    });

    // Execute
    e.as_contract(&contract_address, || {
        execute(&e, targets, functions, args, &desc_hash);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Executed);
    });
}

#[test]
fn full_proposal_lifecycle_to_canceled() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.clone().to_xdr(&e)).to_bytes();

    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets.clone(), functions.clone(), args.clone(), description, &proposer)
    });

    // Cancel while pending
    e.as_contract(&contract_address, || {
        cancel(&e, targets, functions, args, &desc_hash);
        assert_eq!(storage::get_proposal_state(&e, &pid), ProposalState::Canceled);
    });
}
