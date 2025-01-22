#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract, symbol_short,
    testutils::{
        storage::{Instance, Persistent},
        Address as _, Events, Ledger,
    },
    vec, Address, Env, IntoVal,
};

use crate::storage::{
    allowance, approve, balance, burn, mint, set_allowance, spend_allowance, total_supply,
    transfer, transfer_from, update, StorageKey, BALANCE_EXTEND_AMOUNT, INSTANCE_EXTEND_AMOUNT,
};

#[contract]
struct MockContract;

#[test]
fn initial_state() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        assert_eq!(total_supply(&e), 0);
        assert_eq!(balance(&e, &account), 0);
    });
}

#[test]
fn bump_instance_works() {
    let e = Env::default();

    e.ledger().with_mut(|l| {
        // Minimum TTL for persistent entries - new persistent (and instance)
        // entries will have this TTL when created.
        l.min_persistent_entry_ttl = 500;
    });

    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let ttl = e.storage().instance().get_ttl();
        // Note, that TTL doesn't include the current ledger, but when entry
        // is created the current ledger is counted towards the number of
        // ledgers specified by `min_persistent_entry_ttl`, thus
        // the TTL is 1 ledger less than the respective setting.
        assert_eq!(ttl, 499);

        let current = e.ledger().sequence();
        e.ledger().set_sequence_number(current + ttl);

        total_supply(&e);
        assert_eq!(e.storage().instance().get_ttl(), INSTANCE_EXTEND_AMOUNT);
    });
}

#[test]
fn mint_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        assert_eq!(balance(&e, &account), 100);
        assert_eq!(total_supply(&e), 100);
    });
}

#[test]
fn burn_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        burn(&e, &account, 50);
        assert_eq!(balance(&e, &account), 50);
        assert_eq!(total_supply(&e), 50);
    });
}

#[test]
fn approve_with_event() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let allowance_data = (50, 1000);
        approve(&e, &owner, &spender, allowance_data.0, allowance_data.1);
        let allowance_val = allowance(&e, &owner, &spender);
        assert_eq!(allowance_val, 50);

        let events = e.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events,
            vec![
                &e,
                (
                    address.clone(),
                    vec![
                        &e,
                        symbol_short!("approve").into_val(&e),
                        owner.into_val(&e),
                        spender.into_val(&e)
                    ],
                    allowance_data.into_val(&e)
                )
            ]
        );
    });
}

#[test]
fn approve_handles_expiry() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        approve(&e, &owner, &spender, 50, 2);
        e.ledger().set_sequence_number(3);

        let expired_allowance = allowance(&e, &owner, &spender);
        assert_eq!(expired_allowance, 0);
    });
}

#[test]
fn spend_allowance_reduces_value() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        approve(&e, &owner, &spender, 50, 1000);

        spend_allowance(&e, &owner, &spender, 20);

        let updated_allowance = allowance(&e, &owner, &spender);
        assert_eq!(updated_allowance, 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn spend_allowance_insufficient_allowance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        approve(&e, &owner, &spender, 10, 1000);
        spend_allowance(&e, &owner, &spender, 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn set_allowance_with_expired_ledger_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        e.ledger().set_sequence_number(10);
        set_allowance(&e, &owner, &spender, 50, 5, true);
    });
}

#[test]
fn set_allowance_with_zero_value() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        set_allowance(&e, &owner, &spender, 0, 5, false);
        let allowance_val = allowance(&e, &owner, &spender);
        assert_eq!(allowance_val, 0);

        // should pass for a past ledger
        e.ledger().set_sequence_number(10);
        set_allowance(&e, &owner2, &spender, 0, 5, false);
        let allowance_val = allowance(&e, &owner2, &spender);
        assert_eq!(allowance_val, 0);
    });
}

#[test]
fn transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 100);
        transfer(&e, &from, &recipient, 50);
        assert_eq!(balance(&e, &from), 50);
        assert_eq!(balance(&e, &recipient), 50);

        let events = e.events().all();
        assert_eq!(events.len(), 1);
    });
}

#[test]
fn extend_balance_ttl_thru_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 100);

        let key = StorageKey::Balance(from.clone());

        let ttl = e.storage().persistent().get_ttl(&key);
        e.ledger().with_mut(|l| {
            l.sequence_number += ttl;
        });
        transfer(&e, &from, &recipient, 50);
        let ttl = e.storage().persistent().get_ttl(&key);
        assert_eq!(ttl, BALANCE_EXTEND_AMOUNT);
    });
}

#[test]
fn approve_and_transfer_from() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, 50, 1000);

        let allowance_val = allowance(&e, &owner, &spender);
        assert_eq!(allowance_val, 50);

        transfer_from(&e, &spender, &owner, &recipient, 30);
        assert_eq!(balance(&e, &owner), 70);
        assert_eq!(balance(&e, &recipient), 30);

        let updated_allowance = allowance(&e, &owner, &spender);
        assert_eq!(updated_allowance, 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn transfer_insufficient_balance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 50);
        transfer(&e, &from, &recipient, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn transfer_from_insufficient_allowance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, 30, 1000);
        transfer_from(&e, &spender, &owner, &recipient, 50);
    });
}

#[test]
fn update_transfers_between_accounts() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 100);
        update(&e, Some(&from), Some(&to), 50);
        assert_eq!(balance(&e, &from), 50);
        assert_eq!(balance(&e, &to), 50);
    });
}

#[test]
fn update_mints_tokens() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        update(&e, None, Some(&to), 100);
        assert_eq!(balance(&e, &to), 100);
        assert_eq!(total_supply(&e), 100);
    });
}

#[test]
fn update_burns_tokens() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 100);
        update(&e, Some(&from), None, 50);
        assert_eq!(balance(&e, &from), 50);
        assert_eq!(total_supply(&e), 50);
    });
}

#[test]
#[should_panic(expected = "value must be > 0")]
fn update_with_invalid_value_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        update(&e, Some(&from), Some(&to), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn update_with_insufficient_balance_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 50);
        update(&e, Some(&from), Some(&to), 100);
    });
}
