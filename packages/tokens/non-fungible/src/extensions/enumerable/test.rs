#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::{
    approve,
    extensions::enumerable::storage::{
        add_to_global_enumeration, add_to_owner_enumeration, decrement_total_supply,
        get_owner_token_id, get_token_id, increment_total_supply, non_sequential_burn,
        non_sequential_burn_from, non_sequential_mint, remove_from_global_enumeration,
        remove_from_owner_enumeration, sequential_burn, sequential_burn_from, sequential_mint,
        total_supply, transfer, transfer_from,
    },
    StorageKey, TokenId,
};

#[contract]
struct MockContract;

#[test]
fn test_total_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id1 = sequential_mint(&e, &owner);
        let _token_id2 = sequential_mint(&e, &owner);

        assert_eq!(total_supply(&e), 2);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id1);

        // TODO: below fails because the same event is read by the
        // `event_assert`, not the next one. event_assert.
        // assert_non_fungible_mint(&owner, token_id2);
    });
}

#[test]
fn test_get_owner_token_id() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id1 = sequential_mint(&e, &owner);
        let token_id2 = sequential_mint(&e, &owner);

        assert_eq!(get_owner_token_id(&e, &owner, 0), token_id1);
        assert_eq!(get_owner_token_id(&e, &owner, 1), token_id2);
    });
}

#[test]
fn test_get_token_id() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let token_id1 = 42;
    let token_id2 = 83;

    e.as_contract(&address, || {
        non_sequential_mint(&e, &owner, token_id1);
        non_sequential_mint(&e, &owner, token_id2);

        assert_eq!(get_token_id(&e, 0), token_id1);
        assert_eq!(get_token_id(&e, 1), token_id2);
    });
}

#[test]
fn test_sequential_mint() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = sequential_mint(&e, &owner);
        assert_eq!(get_owner_token_id(&e, &owner, 0), token_id);
        assert_eq!(total_supply(&e), 1);
    });
}

#[test]
fn test_non_sequential_mint() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = 42;
        non_sequential_mint(&e, &owner, token_id);
        assert_eq!(get_owner_token_id(&e, &owner, 0), token_id);
        assert_eq!(get_token_id(&e, 0), token_id);
        assert_eq!(total_supply(&e), 1);
    });
}

#[test]
fn test_sequential_burn() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = sequential_mint(&e, &owner);
        sequential_burn(&e, &owner, token_id);
        assert_eq!(total_supply(&e), 0);
    });
}

#[test]
fn test_non_sequential_burn() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = 42;
        non_sequential_mint(&e, &owner, token_id);
        non_sequential_burn(&e, &owner, token_id);
        assert_eq!(total_supply(&e), 0);
    });
}

#[test]
fn test_sequential_burn_from() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = sequential_mint(&e, &owner);
        approve(&e, &owner, &spender, token_id, 1000);
        sequential_burn_from(&e, &spender, &owner, token_id);
        assert_eq!(total_supply(&e), 0);
    });
}

#[test]
fn test_non_sequential_burn_from() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = 42;
        non_sequential_mint(&e, &owner, token_id);
        approve(&e, &owner, &spender, token_id, 1000);
        non_sequential_burn_from(&e, &spender, &owner, token_id);
        assert_eq!(total_supply(&e), 0);
    });
}

#[test]
fn test_increment_total_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let initial_supply = total_supply(&e);
        increment_total_supply(&e);
        assert_eq!(total_supply(&e), initial_supply + 1);
    });
}

#[test]
fn test_decrement_total_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        increment_total_supply(&e);
        let initial_supply = total_supply(&e);
        decrement_total_supply(&e);
        assert_eq!(total_supply(&e), initial_supply - 1);
    });
}

#[test]
fn test_add_to_owner_enumeration() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let token_id = 42;

    e.as_contract(&address, || {
        // simulating mint, transfer, etc. for increasing the balance
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &(1 as TokenId));

        add_to_owner_enumeration(&e, &owner, token_id);
        assert_eq!(get_owner_token_id(&e, &owner, 0), token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #308)")]
fn test_remove_from_owner_enumeration() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &(1 as TokenId));
        let token_id = 42;
        add_to_owner_enumeration(&e, &owner, token_id);
        remove_from_owner_enumeration(&e, &owner, token_id);

        get_owner_token_id(&e, &owner, 0);
    });
}

#[test]
fn test_add_to_global_enumeration() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let token_id = 42;
        let total_supply = increment_total_supply(&e);
        add_to_global_enumeration(&e, token_id, total_supply);
        assert_eq!(get_token_id(&e, 0), token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #309)")]
fn test_remove_from_global_enumeration() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let token_id = 42;
        let total_supply = increment_total_supply(&e);
        add_to_global_enumeration(&e, token_id, total_supply);
        remove_from_global_enumeration(&e, token_id, total_supply);

        get_token_id(&e, 0);
    });
}

#[test]
fn test_enumerable_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = sequential_mint(&e, &owner);
        transfer(&e, &owner, &recipient, token_id);

        assert_eq!(get_owner_token_id(&e, &recipient, 0), token_id);
        assert_eq!(total_supply(&e), 1);
    });
}

#[test]
fn test_enumerable_transfer_from() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = sequential_mint(&e, &owner);
        approve(&e, &owner, &spender, token_id, 1000);
        transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(get_owner_token_id(&e, &recipient, 0), token_id);
        assert_eq!(total_supply(&e), 1);
    });
}
