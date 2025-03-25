#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{
    consecutive::storage::{
        approve, batch_mint, burn, burn_from, owner_of, set_owner_for, transfer, transfer_from,
        StorageKey,
    },
    sequential::next_token_id,
    storage::balance,
};

#[contract]
pub struct MockContract;

#[test]
fn consecutive_batch_mint_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);

        assert_eq!(next_token_id(&e), amount);
        assert_eq!(balance(&e, &owner), amount);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(0)).unwrap();
        assert_eq!(_owner, owner);
        assert_eq!(owner_of(&e, 50), owner);

        // new mint
        let last_id = batch_mint(&e, &owner, amount);
        assert_eq!(last_id, 2 * amount - 1);
        assert_eq!(balance(&e, &owner), 2 * amount);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_owner_of_on_nonexistent_token_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        batch_mint(&e, &user, 5);
        // token 5 is out of range
        owner_of(&e, 5);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_owner_of_panics_on_burnt_token_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        batch_mint(&e, &user, 10);
        burn(&e, &user, 2);
        owner_of(&e, 2);
    });
}

#[test]
fn consecutive_transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        assert_eq!(balance(&e, &owner), amount);

        transfer(&e, &owner, &recipient, 50);
        assert_eq!(owner_of(&e, 50), recipient);
        assert_eq!(balance(&e, &recipient), 1);

        assert_eq!(owner_of(&e, 51), owner);
        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(51)).unwrap();
        assert_eq!(_owner, owner);
    });

    e.as_contract(&address, || {
        transfer(&e, &owner, &recipient, 99);
        assert_eq!(owner_of(&e, 99), recipient);
        assert_eq!(balance(&e, &recipient), 2);
    });
}

#[test]
fn consecutive_transfer_from_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let spender = Address::generate(&e);
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100;
    let token_id = 50;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        assert_eq!(balance(&e, &owner), amount);

        approve(&e, &owner, &spender, token_id, 100);
        transfer_from(&e, &spender, &owner, &recipient, token_id);
        assert_eq!(owner_of(&e, token_id), recipient);
        assert_eq!(balance(&e, &recipient), 1);

        assert_eq!(owner_of(&e, token_id + 1), owner);
    });
}

#[test]
fn consecutive_burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100;
    let token_id = 50;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        assert_eq!(balance(&e, &owner), amount);

        burn(&e, &owner, token_id);
        assert_eq!(balance(&e, &owner), amount - 1);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        let _owner =
            e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id + 1)).unwrap();
        assert_eq!(_owner, owner);
    });
}

#[test]
fn consecutive_burn_from_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let amount = 100;
    let token_id = 42;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        approve(&e, &owner, &spender, token_id, 100);
        burn_from(&e, &spender, &owner, token_id);

        assert_eq!(balance(&e, &owner), amount - 1);
        let burned =
            e.storage().persistent().get::<_, bool>(&StorageKey::BurnedToken(token_id)).unwrap();
        assert!(burned);
        assert_eq!(owner_of(&e, token_id + 1), owner);
    });
}

#[test]
fn consecutive_set_owner_for_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    e.as_contract(&address, || {
        batch_mint(&e, &user1, 5); // 0,1,2,3,4

        // existing id
        set_owner_for(&e, &user2, 2);
        assert_eq!(owner_of(&e, 2), user2);

        // when more than max -> does nothing
        set_owner_for(&e, &user2, 5);
        let owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(5));
        assert_eq!(owner, None);

        // when already has owner -> does nothing
        e.storage().persistent().set(&StorageKey::Owner(3), &user3);
        set_owner_for(&e, &user2, 3);
        assert_eq!(owner_of(&e, 3), user3);

        // when is burned -> does nothing
        burn(&e, &user1, 0);
        set_owner_for(&e, &user2, 0);
        let owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(0));
        assert_eq!(owner, None);
    });
}
