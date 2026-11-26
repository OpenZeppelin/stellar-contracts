extern crate std;

use std::vec::Vec as StdVec;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{storage::Persistent as _, Address as _, Events, Ledger},
    Address, Env,
};

use crate::votes::{
    delegate, get_checkpoint, get_delegate, get_total_supply, get_total_supply_at_checkpoint,
    get_votes, get_votes_at_checkpoint, get_voting_units, num_checkpoints, transfer_voting_units,
    CheckpointType, VotesStorageKey, CHECKPOINTS_PER_CHUNK, VOTES_EXTEND_AMOUNT,
    VOTES_TTL_THRESHOLD,
};

#[contract]
struct MockContract;

#[contract]
struct VotesContract;

#[contractimpl]
impl VotesContract {
    pub fn mint(e: &Env, to: Address, amount: u128) {
        transfer_voting_units(e, None, Some(&to), amount);
    }

    pub fn transfer(e: &Env, from: Address, to: Address, amount: u128) {
        transfer_voting_units(e, Some(&from), Some(&to), amount);
    }
}

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
}

#[test]
fn initial_state_has_zero_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.ledger().set_sequence_number(1000);
    e.as_contract(&contract_address, || {
        assert_eq!(get_votes(&e, &alice), 0);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(num_checkpoints(&e, &alice), 0);
        assert_eq!(get_delegate(&e, &alice), None);
        assert_eq!(get_total_supply(&e), 0);

        // Test get_votes_at_checkpoint returns 0 when num_checkpoints == 0
        assert_eq!(get_votes_at_checkpoint(&e, &alice, 500), 0);

        // Test get_total_supply_at_checkpoint returns 0 when
        // num_total_supply_checkpoints == 0
        assert_eq!(get_total_supply_at_checkpoint(&e, 500), 0);
    });
}

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

        assert_eq!(e.events().all().events().len(), 2);
        assert_eq!(get_delegate(&e, &alice), Some(alice.clone()));
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

        assert_eq!(e.events().all().events().len(), 2);
        assert_eq!(get_delegate(&e, &alice), Some(bob.clone()));
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
    e.ledger().set_sequence_number(1000);

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
    e.ledger().set_sequence_number(1000);

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
        e.ledger().set_sequence_number(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);
        assert_eq!(num_checkpoints(&e, &bob), 1);

        e.ledger().set_sequence_number(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);
        assert_eq!(num_checkpoints(&e, &bob), 2);

        e.ledger().set_sequence_number(3000);
        transfer_voting_units(&e, None, Some(&alice), 25);
        assert_eq!(num_checkpoints(&e, &bob), 3);

        assert_eq!(get_votes(&e, &bob), 175);
    });
}

// ################## HISTORICAL QUERY TESTS ##################

#[test]
fn get_votes_at_checkpoint_returns_historical_value() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_sequence_number(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);

        e.ledger().set_sequence_number(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);

        e.ledger().set_sequence_number(3000);
        transfer_voting_units(&e, None, Some(&alice), 25);

        // Query at different ledgers
        e.ledger().set_sequence_number(4000);

        assert_eq!(get_votes_at_checkpoint(&e, &bob, 999), 0);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 1000), 100);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 1500), 100);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 2000), 150);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 2500), 150);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 3000), 175);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 3500), 175);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_votes_at_checkpoint_fails_for_future_ledger() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    e.ledger().set_sequence_number(1000);

    e.as_contract(&contract_address, || {
        get_votes_at_checkpoint(&e, &alice, 1000);
    });
}

#[test]
fn get_total_supply_at_checkpoint_returns_historical_value() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_sequence_number(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);

        e.ledger().set_sequence_number(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);

        e.ledger().set_sequence_number(3000);
        transfer_voting_units(&e, Some(&alice), None, 30);

        // Query at different ledgers
        e.ledger().set_sequence_number(4000);

        assert_eq!(get_total_supply_at_checkpoint(&e, 999), 0);
        assert_eq!(get_total_supply_at_checkpoint(&e, 1000), 100);
        assert_eq!(get_total_supply_at_checkpoint(&e, 1500), 100);
        assert_eq!(get_total_supply_at_checkpoint(&e, 2000), 150);
        assert_eq!(get_total_supply_at_checkpoint(&e, 2500), 150);
        assert_eq!(get_total_supply_at_checkpoint(&e, 3000), 120);
        assert_eq!(get_total_supply_at_checkpoint(&e, 3500), 120);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_total_supply_at_checkpoint_fails_for_future_ledger() {
    let (e, contract_address) = setup_env();
    e.ledger().set_sequence_number(1000);

    e.as_contract(&contract_address, || {
        get_total_supply_at_checkpoint(&e, 1000);
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

        assert_eq!(get_delegate(&e, &alice), Some(bob.clone()));
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
#[should_panic(expected = "Error(Contract, #4103)")]
fn delegate_to_same_delegate_errors() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.ledger().set_sequence_number(1000);
    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);
    });

    e.ledger().set_sequence_number(2000);
    e.as_contract(&contract_address, || {
        // Should panic with SameDelegateReassignment
        delegate(&e, &alice, &bob);
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
            e.ledger().set_sequence_number(i * 100);
            transfer_voting_units(&e, None, Some(&alice), 10);
        }

        // 1 initial + 20 more = 21 checkpoints
        assert_eq!(num_checkpoints(&e, &bob), 21);

        // Query various historical points
        e.ledger().set_sequence_number(3000);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 50), 1000); // After initial delegation
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 100), 1010);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 500), 1050);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 1000), 1100);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 1500), 1150);
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 2000), 1200);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4102)")]
fn transfer_voting_units_insufficient_balance() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        // Try to transfer more than available
        transfer_voting_units(&e, Some(&alice), Some(&bob), 150);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4101)")]
fn mint_overflow_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), u128::MAX);
        // Try to mint more, causing overflow
        transfer_voting_units(&e, None, Some(&alice), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4101)")]
fn delegate_votes_overflow() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Give alice max voting units and delegate to charlie
        transfer_voting_units(&e, None, Some(&alice), u128::MAX);
        delegate(&e, &alice, &charlie);

        // Give bob 1 voting unit and delegate to charlie - should overflow
        transfer_voting_units(&e, None, Some(&bob), 1);
        delegate(&e, &bob, &charlie);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_votes_at_checkpoint_at_current_timestamp() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    e.ledger().set_sequence_number(1000);

    e.as_contract(&contract_address, || {
        // Query at exact current timestamp should fail
        get_votes_at_checkpoint(&e, &alice, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_total_supply_at_checkpoint_at_current_timestamp() {
    let (e, contract_address) = setup_env();
    e.ledger().set_sequence_number(1000);

    e.as_contract(&contract_address, || {
        // Query at exact current timestamp should fail
        get_total_supply_at_checkpoint(&e, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_votes_at_checkpoint_future_timestamp() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    e.ledger().set_sequence_number(1000);

    e.as_contract(&contract_address, || {
        // Query at future timestamp should fail
        get_votes_at_checkpoint(&e, &alice, 2000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4100)")]
fn get_total_supply_at_checkpoint_future_timestamp() {
    let (e, contract_address) = setup_env();
    e.ledger().set_sequence_number(1000);

    e.as_contract(&contract_address, || {
        // Query at future timestamp should fail
        get_total_supply_at_checkpoint(&e, 2000);
    });
}

#[test]
fn get_votes_at_checkpoint_before_first_checkpoint() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_sequence_number(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &bob);

        e.ledger().set_sequence_number(2000);
        // Query before first checkpoint
        assert_eq!(get_votes_at_checkpoint(&e, &bob, 500), 0);
    });
}

#[test]
fn get_total_supply_at_checkpoint_before_first_checkpoint() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_sequence_number(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);

        e.ledger().set_sequence_number(2000);
        // Query before first checkpoint
        assert_eq!(get_total_supply_at_checkpoint(&e, 500), 0);
    });
}

#[test]
fn transfer_to_self_with_delegation() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 100);
        delegate(&e, &alice, &alice);
        assert_eq!(get_votes(&e, &alice), 100);

        // Transfer to self should be a no-op for votes
        transfer_voting_units(&e, Some(&alice), Some(&alice), 50);
        assert_eq!(get_votes(&e, &alice), 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
}

#[test]
fn total_supply_checkpoint_updates_on_mint_burn() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        e.ledger().set_sequence_number(1000);
        transfer_voting_units(&e, None, Some(&alice), 100);

        e.ledger().set_sequence_number(2000);
        transfer_voting_units(&e, None, Some(&alice), 50);

        e.ledger().set_sequence_number(3000);
        transfer_voting_units(&e, Some(&alice), None, 75);

        e.ledger().set_sequence_number(4000);

        assert_eq!(get_total_supply_at_checkpoint(&e, 1000), 100);
        assert_eq!(get_total_supply_at_checkpoint(&e, 2000), 150);
        assert_eq!(get_total_supply_at_checkpoint(&e, 3000), 75);
        assert_eq!(get_total_supply(&e), 75);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4104)")]
fn get_checkpoint_directly_panics_when_missing() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        get_checkpoint(&e, &CheckpointType::Account(alice.clone()), 0);
    });
}

#[test]
fn checkpoints_fill_a_chunk_before_starting_the_next() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        let chunk_key = |index| VotesStorageKey::DelegateCheckpointChunk(alice.clone(), index);

        e.ledger().set_sequence_number(1);
        transfer_voting_units(&e, None, Some(&alice), 1);
        delegate(&e, &alice, &alice);

        // Fill the first chunk
        for i in 2..=CHECKPOINTS_PER_CHUNK {
            e.ledger().set_sequence_number(i);
            transfer_voting_units(&e, None, Some(&alice), 1);
        }
        assert_eq!(num_checkpoints(&e, &alice), CHECKPOINTS_PER_CHUNK);
        assert!(e.storage().persistent().has(&chunk_key(0)));
        assert!(!e.storage().persistent().has(&chunk_key(1)));

        // The next checkpoint starts a new chunk
        e.ledger().set_sequence_number(CHECKPOINTS_PER_CHUNK + 1);
        transfer_voting_units(&e, None, Some(&alice), 1);
        assert_eq!(num_checkpoints(&e, &alice), CHECKPOINTS_PER_CHUNK + 1);
        assert!(e.storage().persistent().has(&chunk_key(1)));

        // Another change in the same ledger updates the latest checkpoint
        transfer_voting_units(&e, None, Some(&alice), 1);
        assert_eq!(num_checkpoints(&e, &alice), CHECKPOINTS_PER_CHUNK + 1);
        assert_eq!(get_votes(&e, &alice), u128::from(CHECKPOINTS_PER_CHUNK) + 2);
    });
}

#[test]
fn lookups_across_many_chunks_match_history() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    // Enough checkpoints for several full chunks plus a partial one
    let count = 4 * CHECKPOINTS_PER_CHUNK + 3;

    e.as_contract(&contract_address, || {
        // Checkpoint `i` is recorded at ledger `10 * (i + 1)` with `i + 1`
        // votes
        e.ledger().set_sequence_number(10);
        transfer_voting_units(&e, None, Some(&alice), 1);
        delegate(&e, &alice, &alice);
        for i in 1..count {
            e.ledger().set_sequence_number(10 * (i + 1));
            transfer_voting_units(&e, None, Some(&alice), 1);
        }
        assert_eq!(num_checkpoints(&e, &alice), count);

        let cp_type = CheckpointType::Account(alice.clone());
        for i in 0..count {
            let checkpoint = get_checkpoint(&e, &cp_type, i);
            assert_eq!(checkpoint.ledger, 10 * (i + 1));
            assert_eq!(checkpoint.votes, u128::from(i + 1));
        }

        let last_ledger = 10 * count;
        e.ledger().set_sequence_number(last_ledger + 1);
        for ledger in 0..=last_ledger {
            let expected = u128::from(ledger / 10);
            assert_eq!(get_votes_at_checkpoint(&e, &alice, ledger), expected);
            assert_eq!(get_total_supply_at_checkpoint(&e, ledger), expected);
        }
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4104)")]
fn get_checkpoint_panics_past_the_end_of_a_chunk() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        transfer_voting_units(&e, None, Some(&alice), 1);
        delegate(&e, &alice, &alice);
        // Chunk 0 exists but holds a single checkpoint
        get_checkpoint(&e, &CheckpointType::Account(alice.clone()), 1);
    });
}

#[test]
fn lookups_match_history_for_every_size() {
    // Covers 1 to 5 chunks, with the latest chunk holding one, half, all but
    // one, or all of its checkpoints. Alice's votes and the total supply follow
    // different histories (burns and a second holder).
    let sizes = [1, CHECKPOINTS_PER_CHUNK / 2, CHECKPOINTS_PER_CHUNK - 1, CHECKPOINTS_PER_CHUNK];
    let counts = (0..4)
        .flat_map(|full| sizes.map(|size| full * CHECKPOINTS_PER_CHUNK + size))
        .chain([4 * CHECKPOINTS_PER_CHUNK + 1]);
    for count in counts {
        let (e, contract_address) = setup_env();
        let alice = Address::generate(&e);
        let bob = Address::generate(&e);

        // Checkpoint `i` is recorded at ledger `10 * (i + 1)`. Each ledger gets
        // its own invocation to stay within the per-invocation events limit.
        e.ledger().set_sequence_number(10);
        e.as_contract(&contract_address, || {
            transfer_voting_units(&e, None, Some(&alice), 3);
            delegate(&e, &alice, &alice);
            transfer_voting_units(&e, None, Some(&bob), 1);
        });
        for i in 1..count {
            e.ledger().set_sequence_number(10 * (i + 1));
            e.as_contract(&contract_address, || {
                transfer_voting_units(&e, None, Some(&alice), 3);
                if i % 2 == 0 {
                    transfer_voting_units(&e, Some(&alice), None, 1);
                }
            });
        }

        let last_ledger = 10 * count;
        e.ledger().set_sequence_number(last_ledger + 1);
        e.as_contract(&contract_address, || {
            // Every checkpoint ledger and a ledger between each pair
            for ledger in (0..=last_ledger).step_by(5) {
                // After `n` checkpoints, alice has 3n minted minus one burn for
                // every even `i` in 1..n, and bob holds 1 undelegated unit.
                let n = ledger / 10;
                let alice_votes = if n == 0 { 0 } else { u128::from(3 * n - (n - 1) / 2) };
                let supply = if n == 0 { 0 } else { alice_votes + 1 };
                assert_eq!(get_votes_at_checkpoint(&e, &alice, ledger), alice_votes);
                assert_eq!(get_total_supply_at_checkpoint(&e, ledger), supply);
            }
        });
    }
}

// ################## FOOTPRINT STABILITY ##################

/// Builds `count` checkpoints for `account` and for the total supply,
/// checkpoint `i` at ledger `10 * (i + 1)` with `i + 1` votes.
fn build_history(e: &Env, account: &Address, count: u32) {
    e.ledger().set_sequence_number(10);
    transfer_voting_units(e, None, Some(account), 1);
    delegate(e, account, account);
    for i in 1..count {
        e.ledger().set_sequence_number(10 * (i + 1));
        transfer_voting_units(e, None, Some(account), 1);
    }
}

/// Adds `n` checkpoints to `account`'s history and to the total supply, one
/// per ledger, the way a third party minting or sending it single units would.
fn third_party_appends(e: &Env, account: &Address, n: u32) {
    for _ in 0..n {
        e.ledger().set_sequence_number(e.ledger().sequence() + 1);
        transfer_voting_units(e, None, Some(account), 1);
    }
}

fn chunk_key(cp_type: &CheckpointType, index: u32) -> VotesStorageKey {
    match cp_type {
        CheckpointType::TotalSupply => VotesStorageKey::TotalSupplyCheckpointChunk(index),
        CheckpointType::Account(account) => {
            VotesStorageKey::DelegateCheckpointChunk(account.clone(), index)
        }
    }
}

/// Returns the indices of the checkpoint chunks of `cp_type` that `f` reads.
///
/// Reading a chunk extends its TTL, so every chunk's TTL is first refreshed
/// and then aged below the extension threshold. A chunk whose TTL changes
/// while `f` runs was read by `f`. Chunks are only written after being read,
/// so writes show up too. Accesses that do not extend the TTL, such as `has()`,
/// are not detected.
fn chunks_read(e: &Env, cp_type: &CheckpointType, f: impl FnOnce()) -> StdVec<u32> {
    let exists = |index| e.storage().persistent().has(&chunk_key(cp_type, index));
    // One index past the existing chunks, to detect a new chunk being started
    let num_chunks = (0..).find(|&index| !exists(index)).unwrap() + 1;

    for index in 0..num_chunks - 1 {
        get_checkpoint(e, cp_type, index * CHECKPOINTS_PER_CHUNK);
    }
    let age = VOTES_EXTEND_AMOUNT - VOTES_TTL_THRESHOLD + 1;
    e.ledger().set_sequence_number(e.ledger().sequence() + age);

    let ttls = || -> StdVec<Option<u32>> {
        (0..num_chunks)
            .map(|index| {
                exists(index).then(|| e.storage().persistent().get_ttl(&chunk_key(cp_type, index)))
            })
            .collect()
    };
    let before = ttls();
    f();
    let after = ttls();

    (0..num_chunks).filter(|&index| before[index as usize] != after[index as usize]).collect()
}

#[test]
fn historical_reads_touch_the_same_chunks_while_the_latest_chunk_grows() {
    // One snapshot in the first chunk (binary search) and one in the latest
    // chunk (fast path)
    let latest = 2 * CHECKPOINTS_PER_CHUNK + 2;
    for (snapshot, votes) in [(55, 5), (10 * latest + 5, u128::from(latest))] {
        let (e, contract_address) = setup_env();
        let alice = Address::generate(&e);
        let account = CheckpointType::Account(alice.clone());
        let supply = CheckpointType::TotalSupply;

        e.as_contract(&contract_address, || {
            // 3 chunks, the latest holding 3 checkpoints
            build_history(&e, &alice, 2 * CHECKPOINTS_PER_CHUNK + 3);

            let read_votes = || assert_eq!(get_votes_at_checkpoint(&e, &alice, snapshot), votes);
            let read_supply = || assert_eq!(get_total_supply_at_checkpoint(&e, snapshot), votes);
            let simulated =
                (chunks_read(&e, &account, read_votes), chunks_read(&e, &supply, read_supply));

            // A third party fills the latest chunk up to one free slot
            third_party_appends(&e, &alice, CHECKPOINTS_PER_CHUNK - 4);

            let executed =
                (chunks_read(&e, &account, read_votes), chunks_read(&e, &supply, read_supply));
            assert_eq!(simulated, executed);
        });
    }
}

#[test]
fn writes_touch_the_same_chunk_while_it_grows() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let account = CheckpointType::Account(alice.clone());
    let supply = CheckpointType::TotalSupply;

    e.as_contract(&contract_address, || {
        build_history(&e, &alice, CHECKPOINTS_PER_CHUNK + 2);

        // Alice's transfer to bob and a mint to bob (who has no delegate, so
        // only the total supply changes), measured before and after a third
        // party adds checkpoints that stay within the latest chunk
        let transfer = || transfer_voting_units(&e, Some(&alice), Some(&bob), 1);
        let mint = || transfer_voting_units(&e, None, Some(&bob), 1);
        let simulated = (chunks_read(&e, &account, transfer), chunks_read(&e, &supply, mint));
        third_party_appends(&e, &alice, CHECKPOINTS_PER_CHUNK - 4);
        let executed = (chunks_read(&e, &account, transfer), chunks_read(&e, &supply, mint));

        assert_eq!(simulated, (StdVec::from([1]), StdVec::from([1])));
        assert_eq!(executed, simulated);
        assert!(!e.storage().persistent().has(&chunk_key(&account, 2)));
        assert!(!e.storage().persistent().has(&chunk_key(&supply, 2)));
    });
}

#[test]
fn writes_stay_within_the_next_chunk_when_a_new_chunk_starts() {
    // A client that also declares the chunk after the latest one it touches
    // stays within its footprint after `CHECKPOINTS_PER_CHUNK` third-party
    // checkpoints, from a partly full and from a full latest chunk.
    for (count, simulated_chunks) in
        [(CHECKPOINTS_PER_CHUNK + 2, [1].as_slice()), (2 * CHECKPOINTS_PER_CHUNK, &[1, 2])]
    {
        let (e, contract_address) = setup_env();
        let alice = Address::generate(&e);
        let bob = Address::generate(&e);
        let account = CheckpointType::Account(alice.clone());
        let supply = CheckpointType::TotalSupply;

        e.as_contract(&contract_address, || {
            build_history(&e, &alice, count);

            let transfer = || transfer_voting_units(&e, Some(&alice), Some(&bob), 1);
            let mint = || transfer_voting_units(&e, None, Some(&bob), 1);
            let simulated = (chunks_read(&e, &account, transfer), chunks_read(&e, &supply, mint));
            assert_eq!(simulated.0, simulated_chunks);
            assert_eq!(simulated.1, simulated_chunks);

            third_party_appends(&e, &alice, CHECKPOINTS_PER_CHUNK);
            let executed = (chunks_read(&e, &account, transfer), chunks_read(&e, &supply, mint));

            // The executed writes moved on to a new chunk, which is the one
            // declared on top of the simulated chunks
            let next = simulated_chunks.last().unwrap() + 1;
            for chunks in [&executed.0, &executed.1] {
                assert!(chunks.contains(&next));
                assert!(chunks.iter().all(|c| simulated_chunks.contains(c) || *c == next));
            }
        });
    }
}

#[test]
fn historical_reads_can_move_when_a_new_chunk_starts() {
    // Documents the accepted limitation: once a third party starts a new chunk,
    // a historical lookup can read an older chunk it did not read at
    // simulation, even if the client also declared the next chunk.
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let account = CheckpointType::Account(alice.clone());

    e.as_contract(&contract_address, || {
        build_history(&e, &alice, 4 * CHECKPOINTS_PER_CHUNK);

        let read = || assert_eq!(get_votes_at_checkpoint(&e, &alice, 45), 4);
        let simulated = chunks_read(&e, &account, read);
        third_party_appends(&e, &alice, 1);
        let executed = chunks_read(&e, &account, read);

        assert_eq!(simulated, [0, 1, 3]);
        assert_eq!(executed, [0, 1, 2, 4]);
    });
}

#[test]
fn proposal_reads_move_when_a_new_chunk_starts_in_its_ledger() {
    // A proposal reads the proposer's votes at the previous ledger. If a third
    // party starts a new chunk earlier in the same ledger, that read misses the
    // latest chunk and searches the older ones.
    let count = 3 * CHECKPOINTS_PER_CHUNK;
    let snapshot = 10 * count;

    // Returns the chunks read by a proposal in ledger `snapshot + 1`, after
    // `appended` third-party checkpoints in that ledger. Each case uses its own
    // environment, as both happen in the same ledger.
    let proposal_reads = |appended| {
        let (e, contract_address) = setup_env();
        let alice = Address::generate(&e);
        e.as_contract(&contract_address, || {
            // 3 full chunks, the last checkpoint at ledger `snapshot`
            build_history(&e, &alice, count);
            third_party_appends(&e, &alice, appended);
            chunks_read(&e, &CheckpointType::Account(alice.clone()), || {
                assert_eq!(get_votes_at_checkpoint(&e, &alice, snapshot), u128::from(count));
            })
        })
    };

    // Simulated before the third party starts chunk 3, executed after
    assert_eq!(proposal_reads(0), [2]);
    assert_eq!(proposal_reads(1), [0, 1, 2, 3]);
}

#[test]
fn same_ledger_update_when_the_latest_chunk_is_full() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let cp_type = CheckpointType::Account(alice.clone());

    e.as_contract(&contract_address, || {
        // Two full chunks: checkpoint 31 is the last one of chunk 1
        let count = 2 * CHECKPOINTS_PER_CHUNK;
        build_history(&e, &alice, count);

        // A second change in the same ledger updates checkpoint 31 in chunk 1
        // and does not start chunk 2
        transfer_voting_units(&e, None, Some(&alice), 5);
        assert_eq!(num_checkpoints(&e, &alice), count);
        assert_eq!(get_checkpoint(&e, &cp_type, count - 1).votes, u128::from(count) + 5);
        assert!(!e.storage().persistent().has(&chunk_key(&cp_type, 2)));

        // The same in the middle of a later chunk
        third_party_appends(&e, &alice, 5);
        transfer_voting_units(&e, None, Some(&alice), 10);
        assert_eq!(num_checkpoints(&e, &alice), count + 5);
        assert_eq!(get_checkpoint(&e, &cp_type, count + 4).votes, u128::from(count) + 20);
    });
}

#[test]
fn writes_store_the_same_number_of_bytes_however_full_the_chunk_is() {
    // A third party adding checkpoints between simulation and execution must
    // not raise the bytes a prepared write stores, since those are capped by
    // the write-bytes limit recorded at simulation.
    let write_bytes = |count: u32, same_ledger: bool| {
        let e = Env::default();
        e.mock_all_auths();
        let contract_address = e.register(VotesContract, ());
        let client = VotesContractClient::new(&e, &contract_address);
        let alice = Address::generate(&e);
        e.as_contract(&contract_address, || build_history(&e, &alice, count));
        if !same_ledger {
            e.ledger().set_sequence_number(e.ledger().sequence() + 1);
        }
        client.mint(&alice, &1);
        e.cost_estimate().resources().write_bytes
    };

    // A mint to a self-delegated account writes its delegate's and the total
    // supply's chunk and count, from an almost empty to a full latest chunk,
    // and when it starts a new chunk
    let expected = write_bytes(1, false);
    for count in 2..=2 * CHECKPOINTS_PER_CHUNK {
        assert_eq!(write_bytes(count, false), expected, "{count} checkpoints");
        // Updating the latest checkpoint in the same ledger writes as much
        assert_eq!(write_bytes(count, true), expected, "{count} checkpoints, same ledger");
    }
}

#[test]
fn transferring_a_full_balance_writes_as_many_bytes_as_a_partial_one() {
    // Alice transfers 100 units to bob. If a third party sends her 1 unit
    // after simulation, the transfer no longer empties her balance, and it
    // must not write more bytes than when it did.
    let write_bytes = |balance: u128| {
        let e = Env::default();
        e.mock_all_auths();
        let contract_address = e.register(VotesContract, ());
        let client = VotesContractClient::new(&e, &contract_address);
        let alice = Address::generate(&e);
        let bob = Address::generate(&e);
        e.as_contract(&contract_address, || {
            transfer_voting_units(&e, None, Some(&alice), balance);
            transfer_voting_units(&e, None, Some(&bob), 1);
        });
        client.transfer(&alice, &bob, &100);
        e.cost_estimate().resources().write_bytes
    };

    assert_eq!(write_bytes(100), write_bytes(101));
}
