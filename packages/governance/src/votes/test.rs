use soroban_sdk::{contract, testutils::{Address as _, Ledger}, Address, Env};

use crate::votes::{
    delegate, delegates, get_past_total_supply, get_past_votes, get_total_supply, get_votes,
    get_voting_units, num_checkpoints, transfer_voting_units,
};

#[contract]
struct MockContract;

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
}

// ################## BASIC FUNCTIONALITY TESTS ##################

#[test]
fn initial_state_has_zero_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        assert_eq!(get_votes(&e, &alice), 0);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(num_checkpoints(&e, &alice), 0);
        assert_eq!(delegates(&e, &alice), None);
    });
}

#[test]
fn initial_total_supply_is_zero() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        assert_eq!(get_total_supply(&e), 0);
    });
}

// ################## TRANSFER VOTING UNITS TESTS ##################

#[test]
fn mint_increases_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);

        assert_eq!(get_voting_units(&e, &alice), 100);
        assert_eq!(get_total_supply(&e), 100);
    });
}

#[test]
fn mint_does_not_create_votes_without_delegation() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);

        assert_eq!(get_voting_units(&e, &alice), 100);
        assert_eq!(get_votes(&e, &alice), 0);
    });
}

#[test]
fn burn_decreases_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        transfer_voting_units(&e, Some(&alice), None, 30);

        assert_eq!(get_voting_units(&e, &alice), 70);
        assert_eq!(get_total_supply(&e), 70);
    });
}

#[test]
fn transfer_moves_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        transfer_voting_units(&e, Some(&alice), Some(&bob), 40);

        assert_eq!(get_voting_units(&e, &alice), 60);
        assert_eq!(get_voting_units(&e, &bob), 40);
        assert_eq!(get_total_supply(&e), 100);
    });
}

#[test]
fn zero_transfer_is_noop() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 0);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(get_total_supply(&e), 0);
    });
}

// ################## DELEGATION TESTS ##################

#[test]
fn delegate_to_self() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &alice);

        assert_eq!(delegates(&e, &alice), Some(alice.clone()));
        assert_eq!(get_votes(&e, &alice), 100);
    });
}

#[test]
fn delegate_to_other() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);

        assert_eq!(delegates(&e, &alice), Some(bob.clone()));
        assert_eq!(get_votes(&e, &alice), 0);
        assert_eq!(get_votes(&e, &bob), 100);
    });
}

#[test]
fn change_delegate() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);
        assert_eq!(get_votes(&e, &bob), 100);
        assert_eq!(get_votes(&e, &charlie), 0);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &charlie);
        assert_eq!(get_votes(&e, &bob), 0);
        assert_eq!(get_votes(&e, &charlie), 100);
    });
}

#[test]
fn multiple_delegators_to_same_delegate() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        transfer_voting_units(&e, None, Some(&bob), 50);

        delegate(&e, &alice, &charlie);
        delegate(&e, &bob, &charlie);

        assert_eq!(get_votes(&e, &charlie), 150);
    });
}

#[test]
fn transfer_updates_delegate_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let delegate_a = Address::generate(&e);
    let delegate_b = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &delegate_a);
        assert_eq!(get_votes(&e, &delegate_a), 100);

        transfer_voting_units(&e, None, Some(&bob), 50);
        delegate(&e, &bob, &delegate_b);
        assert_eq!(get_votes(&e, &delegate_b), 50);

        // Transfer from alice to bob
        transfer_voting_units(&e, Some(&alice), Some(&bob), 30);

        assert_eq!(get_votes(&e, &delegate_a), 70);
        assert_eq!(get_votes(&e, &delegate_b), 80);
    });
}

// ################## CHECKPOINT TESTS ##################

#[test]
fn checkpoints_created_on_delegation() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);

        assert_eq!(num_checkpoints(&e, &bob), 1);
    });
}

#[test]
fn multiple_operations_same_timestamp_single_checkpoint() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);

        transfer_voting_units(&e, None, Some(&alice), 50);

        // Should still have only 1 checkpoint since same timestamp
        assert_eq!(num_checkpoints(&e, &bob), 1);
        assert_eq!(get_votes(&e, &bob), 150);
    });
}

#[test]
fn different_timestamps_create_new_checkpoints() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_timestamp(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);
        assert_eq!(num_checkpoints(&e, &bob), 1);

        e.ledger().set_timestamp(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);
        assert_eq!(num_checkpoints(&e, &bob), 2);

        e.ledger().set_timestamp(3000);
        transfer_voting_units(&e, None, Some(&alice), 25);
        assert_eq!(num_checkpoints(&e, &bob), 3);

        assert_eq!(get_votes(&e, &bob), 175);
    });
}

// ################## HISTORICAL QUERY TESTS ##################

#[test]
fn get_past_votes_returns_historical_value() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_timestamp(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);

        e.ledger().set_timestamp(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);

        e.ledger().set_timestamp(3000);
        transfer_voting_units(&e, None, Some(&alice), 25);

        // Query at different timepoints
        e.ledger().set_timestamp(4000);

        assert_eq!(get_past_votes(&e, &bob, 999), 0);
        assert_eq!(get_past_votes(&e, &bob, 1000), 100);
        assert_eq!(get_past_votes(&e, &bob, 1500), 100);
        assert_eq!(get_past_votes(&e, &bob, 2000), 150);
        assert_eq!(get_past_votes(&e, &bob, 2500), 150);
        assert_eq!(get_past_votes(&e, &bob, 3000), 175);
        assert_eq!(get_past_votes(&e, &bob, 3500), 175);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_past_votes_fails_for_future_timepoint() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        get_past_votes(&e, &alice, 1000);
    });
}

#[test]
fn get_past_total_supply_returns_historical_value() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_timestamp(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);

        e.ledger().set_timestamp(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);

        e.ledger().set_timestamp(3000);
        transfer_voting_units(&e, Some(&alice), None, 30);

        // Query at different timepoints
        e.ledger().set_timestamp(4000);

        assert_eq!(get_past_total_supply(&e, 999), 0);
        assert_eq!(get_past_total_supply(&e, 1000), 100);
        assert_eq!(get_past_total_supply(&e, 1500), 100);
        assert_eq!(get_past_total_supply(&e, 2000), 150);
        assert_eq!(get_past_total_supply(&e, 2500), 150);
        assert_eq!(get_past_total_supply(&e, 3000), 120);
        assert_eq!(get_past_total_supply(&e, 3500), 120);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_past_total_supply_fails_for_future_timepoint() {
    let (e, contract_address) = setup_env();
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        get_past_total_supply(&e, 1000);
    });
}

// ################## EDGE CASES ##################

#[test]
fn delegate_without_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);

        assert_eq!(delegates(&e, &alice), Some(bob.clone()));
        assert_eq!(get_votes(&e, &bob), 0);
    });
}

#[test]
fn receive_voting_units_after_delegation() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        assert_eq!(get_votes(&e, &bob), 0);

        transfer_voting_units(&e, None, Some(&alice), 100);
        assert_eq!(get_votes(&e, &bob), 100);
    });
}

#[test]
fn burn_all_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);
        assert_eq!(get_votes(&e, &bob), 100);

        transfer_voting_units(&e, Some(&alice), None, 100);
        assert_eq!(get_votes(&e, &bob), 0);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(get_total_supply(&e), 0);
    });
}

#[test]
fn delegate_to_same_delegate_is_noop() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.ledger().set_timestamp(1000);
    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);
    });

    let checkpoints_before = e.as_contract(&contract_address, || {
        num_checkpoints(&e, &bob)
    });

    e.ledger().set_timestamp(2000);
    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        // No new checkpoints should be created since votes didn't change
        assert_eq!(num_checkpoints(&e, &bob), checkpoints_before);
    });
}

#[test]
fn large_voting_power() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        let large_amount: u128 = u128::MAX / 2;
        transfer_voting_units(&e, None, Some(&alice), large_amount);
        delegate(&e, &alice, &bob);

        assert_eq!(get_votes(&e, &bob), large_amount);
        assert_eq!(get_total_supply(&e), large_amount);
    });
}

#[test]
fn binary_search_with_many_checkpoints() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Initial delegation at timestamp 0 creates first checkpoint
        transfer_voting_units(&e, None, Some(&alice), 1000);
        delegate(&e, &alice, &bob);

        // Create many more checkpoints (starting at timestamp 100)
        for i in 1..=20 {
            e.ledger().set_timestamp(i * 100);
            transfer_voting_units(&e, None, Some(&alice), 10);
        }

        // 1 initial + 20 more = 21 checkpoints
        assert_eq!(num_checkpoints(&e, &bob), 21);

        // Query various historical points
        e.ledger().set_timestamp(3000);
        assert_eq!(get_past_votes(&e, &bob, 50), 1000);  // After initial delegation
        assert_eq!(get_past_votes(&e, &bob, 100), 1010);
        assert_eq!(get_past_votes(&e, &bob, 500), 1050);
        assert_eq!(get_past_votes(&e, &bob, 1000), 1100);
        assert_eq!(get_past_votes(&e, &bob, 1500), 1150);
        assert_eq!(get_past_votes(&e, &bob, 2000), 1200);
    });
}
