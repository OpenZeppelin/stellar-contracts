//! # Counting Tests
//!
//! This module contains tests for the Counting module.

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    Address, BytesN, Env,
};

use crate::counting::{
    storage::{
        count_vote, counting_mode, get_proposal_vote_counts, get_quorum, has_voted, quorum_reached,
        set_quorum, tally_succeeded,
    },
    VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR,
};

#[contract]
struct MockContract;

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
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
#[should_panic(expected = "Error(Contract, #4202)")]
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
#[should_panic(expected = "Error(Contract, #4202)")]
fn quorum_reached_fails_when_quorum_not_set() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        quorum_reached(&e, &pid);
    });
}

// ################## ERROR TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
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
#[should_panic(expected = "Error(Contract, #4200)")]
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
#[should_panic(expected = "Error(Contract, #4201)")]
fn count_vote_fails_on_invalid_vote_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, 3, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4201)")]
fn count_vote_fails_on_large_invalid_vote_type() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &alice, u32::MAX, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4203)")]
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
#[should_panic(expected = "Error(Contract, #4203)")]
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
#[should_panic(expected = "Error(Contract, #4203)")]
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
