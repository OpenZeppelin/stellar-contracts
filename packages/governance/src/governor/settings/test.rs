use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, String,
};

use crate::governor::{
    counting_mode, get_name, get_proposal_threshold, get_quorum, get_token_contract, get_version,
    get_voting_delay, get_voting_period, set_name, set_proposal_threshold, set_quorum,
    set_token_contract, set_version, set_voting_delay, set_voting_period, test::setup_env,
};

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
#[should_panic(expected = "Error(Contract, #4218)")]
fn get_quorum_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_quorum(&e, e.ledger().sequence());
    });
}

#[test]
fn set_and_get_quorum() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 1000);
        assert_eq!(get_quorum(&e, e.ledger().sequence()), 1000);
    });
}

#[test]
fn update_quorum() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_quorum(&e, 1000);
        assert_eq!(get_quorum(&e, e.ledger().sequence()), 1000);

        set_quorum(&e, 2000);
        assert_eq!(get_quorum(&e, e.ledger().sequence()), 2000);
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
        assert_eq!(get_quorum(&e, e.ledger().sequence()), 0);
    });
}

#[test]
fn quorum_checkpoint_returns_historical_value() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        // Set quorum to 500 at ledger 100.
        set_quorum(&e, 500);

        // Advance to ledger 200 and update quorum to 1000.
        e.ledger().set_sequence_number(200);
        set_quorum(&e, 1000);

        // Advance to ledger 300 and update quorum to 2000.
        e.ledger().set_sequence_number(300);
        set_quorum(&e, 2000);

        // Historical lookups return the value in effect at each ledger.
        assert_eq!(get_quorum(&e, 100), 500);
        assert_eq!(get_quorum(&e, 150), 500); // between checkpoints
        assert_eq!(get_quorum(&e, 200), 1000);
        assert_eq!(get_quorum(&e, 250), 1000);
        assert_eq!(get_quorum(&e, 300), 2000);
        assert_eq!(get_quorum(&e, 999), 2000); // future ledger uses latest
    });
}

#[test]
fn quorum_checkpoint_same_ledger_updates_in_place() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        // Multiple updates at the same ledger should overwrite, not append.
        set_quorum(&e, 500);
        set_quorum(&e, 1000);
        set_quorum(&e, 2000);

        assert_eq!(get_quorum(&e, e.ledger().sequence()), 2000);

        // Advance and set again — should create a second checkpoint.
        e.ledger().set_sequence_number(200);
        set_quorum(&e, 3000);

        // Original ledger still returns the final in-place value.
        assert_eq!(get_quorum(&e, 100), 2000);
        assert_eq!(get_quorum(&e, 200), 3000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4218)")]
fn quorum_checkpoint_before_first_checkpoint_panics() {
    let (e, contract_address) = setup_env();

    e.ledger().set_sequence_number(100);

    e.as_contract(&contract_address, || {
        // Set quorum at ledger 100.
        set_quorum(&e, 500);

        // Querying before the first checkpoint should panic.
        get_quorum(&e, 50);
    });
}

// ################## CONFIG GETTER/SETTER TESTS ##################

#[test]
fn set_and_get_name() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_name(&e, String::from_str(&e, "MyGovernor"));
        assert_eq!(get_name(&e), String::from_str(&e, "MyGovernor"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4213)")]
fn get_name_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_name(&e);
    });
}

#[test]
fn set_and_get_version() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_version(&e, String::from_str(&e, "1.0.0"));
        assert_eq!(get_version(&e), String::from_str(&e, "1.0.0"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4214)")]
fn get_version_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_version(&e);
    });
}

#[test]
fn set_and_get_proposal_threshold() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_proposal_threshold(&e, 500);
        assert_eq!(get_proposal_threshold(&e), 500);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4212)")]
fn get_proposal_threshold_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_proposal_threshold(&e);
    });
}

#[test]
fn set_and_get_voting_delay() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_voting_delay(&e, 10);
        assert_eq!(get_voting_delay(&e), 10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4210)")]
fn get_voting_delay_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_voting_delay(&e);
    });
}

#[test]
fn set_and_get_voting_period() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_voting_period(&e, 100);
        assert_eq!(get_voting_period(&e), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4211)")]
fn get_voting_period_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_voting_period(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4223)")]
fn set_voting_period_rejects_zero() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_voting_period(&e, 0);
    });
}

#[test]
fn set_voting_period_accepts_minimum() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_voting_period(&e, 1);
        assert_eq!(get_voting_period(&e), 1);
    });
}

#[test]
fn update_config_values() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        set_voting_delay(&e, 10);
        set_voting_period(&e, 100);
        set_proposal_threshold(&e, 500);

        set_voting_delay(&e, 20);
        set_voting_period(&e, 200);
        set_proposal_threshold(&e, 1000);

        assert_eq!(get_voting_delay(&e), 20);
        assert_eq!(get_voting_period(&e), 200);
        assert_eq!(get_proposal_threshold(&e), 1000);
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
#[should_panic(expected = "Error(Contract, #4220)")]
fn get_token_contract_fails_when_not_set() {
    let (e, contract_address) = setup_env();

    e.as_contract(&contract_address, || {
        get_token_contract(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4219)")]
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
#[should_panic(expected = "Error(Contract, #4219)")]
fn set_token_contract_fails_on_same_address() {
    let (e, contract_address) = setup_env();
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_token_contract(&e, &token);
        set_token_contract(&e, &token);
    });
}
