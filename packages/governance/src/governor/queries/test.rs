use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, IntoVal, String, Symbol, Val, Vec,
};

use crate::governor::{
    cast_vote, count_vote, get_proposal_deadline, get_proposal_proposer, get_proposal_snapshot,
    get_proposal_state, get_proposal_vote_counts, has_voted, hash_proposal, propose,
    quorum_reached, set_quorum, set_voting_delay, tally_succeeded,
    test::{
        default_state, proposal_id, set_mock_voting_power, setup_env, setup_env_with_token,
        setup_governor_config, simple_proposal, store_proposal_with_quorum, stored_settings,
    },
    GovernorSettingsStorageKey, ProposalState, VOTE_ABSTAIN, VOTE_AGAINST, VOTE_FOR,
};

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

// ################## PROPOSAL STATE TESTS ##################

#[test]
fn quorum_change_does_not_affect_past_proposals() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    // Quorum is 100 (set by setup_governor_config at ledger 100).
    let pid = e.as_contract(&contract_address, || {
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Advance to voting window and cast 150 votes (passes quorum of 100).
    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    let voter = Address::generate(&e);
    e.as_contract(&contract_address, || {
        count_vote(&e, &pid, &voter, VOTE_FOR, 150);
    });

    // Advance past voting end.
    let deadline = e.as_contract(&contract_address, || get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    // Proposal should be Succeeded (150 >= quorum of 100).
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Succeeded,);
    });

    // Now change quorum to 500 at the current ledger.
    e.as_contract(&contract_address, || {
        set_quorum(&e, 500);
    });

    // The proposal's quorum is still evaluated at vote_snapshot, where
    // quorum was 100. It should still be Succeeded, not Defeated.
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Succeeded,);
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
        store_proposal_with_quorum(&e, &pid);
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);

        assert!(quorum_reached(&e, &pid, 100));
    });
}

#[test]
fn quorum_reached_with_abstain_votes_only() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        store_proposal_with_quorum(&e, &pid);
        count_vote(&e, &pid, &alice, VOTE_ABSTAIN, 100);

        assert!(quorum_reached(&e, &pid, 100));
    });
}

#[test]
fn quorum_reached_with_for_and_abstain_combined() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        store_proposal_with_quorum(&e, &pid);
        count_vote(&e, &pid, &alice, VOTE_FOR, 60);
        count_vote(&e, &pid, &bob, VOTE_ABSTAIN, 40);

        // 60 + 40 = 100 >= 100
        assert!(quorum_reached(&e, &pid, 100));
    });
}

#[test]
fn quorum_not_reached_when_insufficient() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        store_proposal_with_quorum(&e, &pid);
        count_vote(&e, &pid, &alice, VOTE_FOR, 99);

        assert!(!quorum_reached(&e, &pid, 100));
    });
}

#[test]
fn quorum_ignores_against_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        store_proposal_with_quorum(&e, &pid);
        count_vote(&e, &pid, &alice, VOTE_AGAINST, 200);
        count_vote(&e, &pid, &bob, VOTE_FOR, 50);

        // Only for + abstain count toward quorum: 50 < 100
        assert!(!quorum_reached(&e, &pid, 100));
    });
}

#[test]
fn quorum_reached_exactly_at_threshold() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        store_proposal_with_quorum(&e, &pid);
        count_vote(&e, &pid, &alice, VOTE_FOR, 100);

        // Exactly at threshold: 100 >= 100
        assert!(quorum_reached(&e, &pid, 100));
    });
}

#[test]
fn quorum_zero_always_reached() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        store_proposal_with_quorum(&e, &pid);

        // 0 >= 0 with no votes
        assert!(quorum_reached(&e, &pid, 0));
    });
}

#[test]
fn quorum_not_reached_for_nonexistent_proposal() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 1);

    e.as_contract(&contract_address, || {
        // A non-existent proposal has zero votes, so quorum is not reached.
        assert!(!quorum_reached(&e, &pid, 100));
    });
}

// ################## HASH PROPOSAL TESTS ##################

#[test]
fn hash_proposal_is_deterministic() {
    let (e, _) = setup_env();
    let (targets, functions, args, description) = simple_proposal(&e);
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();

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
    let hash1_bytes = e.crypto().keccak256(&desc1.to_bytes()).to_bytes();
    let hash2_bytes = e.crypto().keccak256(&desc2.to_bytes()).to_bytes();

    let id1 = hash_proposal(&e, &targets, &functions, &args, &hash1_bytes);
    let id2 = hash_proposal(&e, &targets, &functions, &args, &hash2_bytes);

    assert_ne!(id1, id2);
}

#[test]
fn hash_proposal_differs_with_different_targets() {
    let (e, _) = setup_env();
    let description = String::from_str(&e, "Test");
    let desc_hash = e.crypto().keccak256(&description.to_bytes()).to_bytes();
    let functions = vec![&e, Symbol::new(&e, "do_something")];
    let args: Vec<Vec<Val>> = vec![&e, vec![&e, 1u32.into_val(&e)]];

    let targets1 = vec![&e, Address::generate(&e)];
    let targets2 = vec![&e, Address::generate(&e)];

    let id1 = hash_proposal(&e, &targets1, &functions, &args, &desc_hash);
    let id2 = hash_proposal(&e, &targets2, &functions, &args, &desc_hash);

    assert_ne!(id1, id2);
}

// ################## PROPOSAL QUERY TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
fn get_proposal_state_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        get_proposal_state(&e, &pid, false);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
fn get_proposal_snapshot_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        get_proposal_snapshot(&e, &pid);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
fn get_proposal_deadline_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        get_proposal_deadline(&e, &pid);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4200)")]
fn get_proposal_proposer_fails_for_nonexistent() {
    let (e, contract_address) = setup_env();
    let pid = proposal_id(&e, 99);

    e.as_contract(&contract_address, || {
        get_proposal_proposer(&e, &pid);
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
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Advance past voting delay (vote_snapshot). Voting opens after snapshot.
    let snapshot = e.as_contract(&contract_address, || get_proposal_snapshot(&e, &pid));
    e.ledger().set_sequence_number(snapshot + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Active);
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
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Advance past deadline
    let deadline = e.as_contract(&contract_address, || get_proposal_deadline(&e, &pid));
    e.ledger().set_sequence_number(deadline + 1);

    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Defeated);
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
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    // Still within voting delay
    e.as_contract(&contract_address, || {
        assert_eq!(default_state(&e, &pid), ProposalState::Pending);
    });
}

/// A zero voting delay is valid: the snapshot lands on the creation ledger,
/// which is still `Pending`, and voting opens on the very next ledger with
/// the full voting period intact.
#[test]
fn zero_voting_delay_opens_voting_on_the_next_ledger() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);
    let created_at = e.ledger().sequence();

    let pid = e.as_contract(&contract_address, || {
        set_voting_delay(&e, 0);
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    e.as_contract(&contract_address, || {
        assert_eq!(get_proposal_snapshot(&e, &pid), created_at);
        assert_eq!(get_proposal_deadline(&e, &pid), created_at + 100);
        assert_eq!(default_state(&e, &pid), ProposalState::Pending);

        e.ledger().set_sequence_number(created_at + 1);
        assert_eq!(default_state(&e, &pid), ProposalState::Active);

        let voter = Address::generate(&e);
        cast_vote(
            &e,
            &pid,
            VOTE_FOR,
            &String::from_str(&e, "yes"),
            &voter,
            default_state(&e, &pid),
        );

        e.ledger().set_sequence_number(created_at + 101);
        assert_eq!(default_state(&e, &pid), ProposalState::Succeeded);
    });
}

/// Regression guard for the deadlock that [`set_voting_period`]
/// rejects. The zero is written straight to storage to bypass that check,
/// showing why the check has to exist: `vote_end` collapses onto
/// `vote_snapshot`, so no ledger can satisfy the `Active` branch and no vote
/// can ever be cast.
#[test]
fn zero_voting_period_leaves_proposal_unvotable() {
    let (e, contract_address, token_address) = setup_env_with_token();
    setup_governor_config(&e, &contract_address);
    set_mock_voting_power(&e, &token_address, 1000);

    let proposer = Address::generate(&e);
    let (targets, functions, args, description) = simple_proposal(&e);

    let pid = e.as_contract(&contract_address, || {
        e.storage().instance().set(&GovernorSettingsStorageKey::VotingPeriod, &0u32);
        propose(&e, targets, functions, args, description, &proposer, &stored_settings(&e))
    });

    e.as_contract(&contract_address, || {
        let snapshot = get_proposal_snapshot(&e, &pid);
        assert_eq!(get_proposal_deadline(&e, &pid), snapshot);

        for offset in 0..30 {
            e.ledger().set_sequence_number(snapshot - 10 + offset);
            assert_ne!(default_state(&e, &pid), ProposalState::Active);
        }

        e.ledger().set_sequence_number(snapshot + 1);
        assert_eq!(default_state(&e, &pid), ProposalState::Defeated);
    });
}
