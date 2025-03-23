#![allow(unused_variables)]
#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_event_assertion::EventAssertion;

use crate::{
    approve,
    extensions::enumerable::storage::{
        add_to_global_enumeration, add_to_owner_enumeration, burn, burn_from,
        decrement_total_supply, get_owner_token_id, get_token_id, increment_total_supply,
        non_sequential_mint, remove_from_global_enumeration, remove_from_owner_enumeration,
        sequential_mint, total_supply, transfer, transfer_from,
    },
    NonFungibleToken, StorageKey,
};

use super::{NonFungibleEnumerable, NonSequential, Sequential};

mod sequential_contract_test {
    use crate::sequential::NonFungibleSequential;

    use super::*;

    #[contract]
    struct MockContractSequential;

    #[contractimpl]
    impl NonFungibleToken for MockContractSequential {
        fn balance(e: &Env, owner: Address) -> u32 {
            crate::storage::balance::<Self>(e, &owner)
        }

        fn owner_of(e: &Env, token_id: u32) -> Address {
            crate::storage::owner_of::<Self>(e, token_id)
        }

        fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
            unimplemented!()
        }

        fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
            unimplemented!()
        }

        fn approve(
            e: &Env,
            approver: Address,
            approved: Address,
            token_id: u32,
            live_until_ledger: u32,
        ) {
            unimplemented!()
        }

        fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
            unimplemented!()
        }

        fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
            crate::storage::get_approved::<Self>(e, token_id)
        }

        fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
            crate::storage::is_approved_for_all::<Self>(e, &owner, &operator)
        }

        fn name(e: &Env) -> String {
            unimplemented!()
        }

        fn symbol(e: &Env) -> String {
            unimplemented!()
        }

        fn token_uri(e: &Env, token_id: u32) -> String {
            unimplemented!()
        }
    }

    impl NonFungibleEnumerable for MockContractSequential {
        type EnumerationStrategy = Sequential;

        fn total_supply(e: &Env) -> u32 {
            unimplemented!()
        }

        fn get_token_id(e: &Env, index: u32) -> u32 {
            unimplemented!()
        }

        fn get_owner_token_id(e: &Env, owner: &Address, index: u32) -> u32 {
            unimplemented!()
        }
    }

    impl NonFungibleSequential for MockContractSequential {
        fn next_token_id(e: &Env) -> u32 {
            crate::sequential::next_token_id::<Self>(e)
        }

        fn increment_token_id(e: &Env, amount: u32) -> u32 {
            crate::sequential::increment_token_id::<Self>(e, amount)
        }
    }

    #[test]
    fn test_total_supply() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id1 = sequential_mint::<MockContractSequential>(&e, &owner);
            let _token_id2 = sequential_mint::<MockContractSequential>(&e, &owner);

            assert_eq!(total_supply::<MockContractSequential>(&e), 2);

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
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id1 = sequential_mint::<MockContractSequential>(&e, &owner);
            let token_id2 = sequential_mint::<MockContractSequential>(&e, &owner);

            assert_eq!(get_owner_token_id::<MockContractSequential>(&e, &owner, 0), token_id1);
            assert_eq!(get_owner_token_id::<MockContractSequential>(&e, &owner, 1), token_id2);
        });
    }

    #[test]
    fn test_sequential_mint() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = sequential_mint::<MockContractSequential>(&e, &owner);
            assert_eq!(get_owner_token_id::<MockContractSequential>(&e, &owner, 0), token_id);
            assert_eq!(total_supply::<MockContractSequential>(&e), 1);
        });
    }

    #[test]
    fn test_sequential_burn() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = sequential_mint::<MockContractSequential>(&e, &owner);
            burn::<MockContractSequential>(&e, &owner, token_id);
            assert_eq!(total_supply::<MockContractSequential>(&e), 0);
        });
    }

    #[test]
    fn test_sequential_burn_from() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = sequential_mint::<MockContractSequential>(&e, &owner);
            approve::<MockContractSequential>(&e, &owner, &spender, token_id, 1000);
            burn_from::<MockContractSequential>(&e, &spender, &owner, token_id);
            assert_eq!(total_supply::<MockContractSequential>(&e), 0);
        });
    }

    #[test]
    fn test_increment_total_supply() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());

        e.as_contract(&address, || {
            let initial_supply = total_supply::<MockContractSequential>(&e);
            increment_total_supply::<MockContractSequential>(&e);
            assert_eq!(total_supply::<MockContractSequential>(&e), initial_supply + 1);
        });
    }

    #[test]
    fn test_add_to_owner_enumeration() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);
        let token_id = 42;

        e.as_contract(&address, || {
            // simulating mint, transfer, etc. for increasing the balance
            e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u32);

            add_to_owner_enumeration::<MockContractSequential>(&e, &owner, token_id);
            assert_eq!(get_owner_token_id::<MockContractSequential>(&e, &owner, 0), token_id);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #307)")]
    fn test_remove_from_owner_enumeration() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u32);
            let token_id = 42;
            add_to_owner_enumeration::<MockContractSequential>(&e, &owner, token_id);
            remove_from_owner_enumeration::<MockContractSequential>(&e, &owner, token_id);

            get_owner_token_id::<MockContractSequential>(&e, &owner, 0);
        });
    }

    #[test]
    fn test_transfer() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);
        let recipient = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = sequential_mint::<MockContractSequential>(&e, &owner);
            transfer::<MockContractSequential>(&e, &owner, &recipient, token_id);

            assert_eq!(get_owner_token_id::<MockContractSequential>(&e, &recipient, 0), token_id);
            assert_eq!(total_supply::<MockContractSequential>(&e), 1);
        });
    }

    #[test]
    fn test_transfer_from() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractSequential, ());
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let recipient = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = sequential_mint::<MockContractSequential>(&e, &owner);
            approve::<MockContractSequential>(&e, &owner, &spender, token_id, 1000);
            transfer_from::<MockContractSequential>(&e, &spender, &owner, &recipient, token_id);

            assert_eq!(get_owner_token_id::<MockContractSequential>(&e, &recipient, 0), token_id);
            assert_eq!(total_supply::<MockContractSequential>(&e), 1);
        });
    }
}

mod non_sequential_contract_test {
    use super::*;

    #[contract]
    struct MockContractNonSequential;

    #[contractimpl]
    impl NonFungibleToken for MockContractNonSequential {
        fn balance(e: &Env, owner: Address) -> u32 {
            crate::storage::balance::<Self>(e, &owner)
        }

        fn owner_of(e: &Env, token_id: u32) -> Address {
            crate::storage::owner_of::<Self>(e, token_id)
        }

        fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
            unimplemented!()
        }

        fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
            unimplemented!()
        }

        fn approve(
            e: &Env,
            approver: Address,
            approved: Address,
            token_id: u32,
            live_until_ledger: u32,
        ) {
            unimplemented!()
        }

        fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
            unimplemented!()
        }

        fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
            crate::storage::get_approved::<Self>(e, token_id)
        }

        fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
            crate::storage::is_approved_for_all::<Self>(e, &owner, &operator)
        }

        fn name(e: &Env) -> String {
            unimplemented!()
        }

        fn symbol(e: &Env) -> String {
            unimplemented!()
        }

        fn token_uri(e: &Env, token_id: u32) -> String {
            unimplemented!()
        }
    }

    impl NonFungibleEnumerable for MockContractNonSequential {
        type EnumerationStrategy = NonSequential;

        fn total_supply(e: &Env) -> u32 {
            crate::enumerable::storage::total_supply::<Self>(e)
        }

        fn get_token_id(e: &Env, index: u32) -> u32 {
            unimplemented!()
        }

        fn get_owner_token_id(e: &Env, owner: &Address, index: u32) -> u32 {
            unimplemented!()
        }
    }

    #[test]
    fn test_get_token_id() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());
        let owner = Address::generate(&e);
        let token_id1 = 42;
        let token_id2 = 83;

        e.as_contract(&address, || {
            non_sequential_mint::<MockContractNonSequential>(&e, &owner, token_id1);
            non_sequential_mint::<MockContractNonSequential>(&e, &owner, token_id2);

            assert_eq!(get_token_id::<MockContractNonSequential>(&e, 0), token_id1);
            assert_eq!(get_token_id::<MockContractNonSequential>(&e, 1), token_id2);
        });
    }

    #[test]
    fn test_non_sequential_mint() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = 42;
            non_sequential_mint::<MockContractNonSequential>(&e, &owner, token_id);
            assert_eq!(get_owner_token_id::<MockContractNonSequential>(&e, &owner, 0), token_id);
            assert_eq!(get_token_id::<MockContractNonSequential>(&e, 0), token_id);
            assert_eq!(total_supply::<MockContractNonSequential>(&e), 1);
        });
    }

    #[test]
    fn test_burn() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());
        let owner = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = 42;
            non_sequential_mint::<MockContractNonSequential>(&e, &owner, token_id);
            burn::<MockContractNonSequential>(&e, &owner, token_id);
            assert_eq!(total_supply::<MockContractNonSequential>(&e), 0);
        });
    }

    #[test]
    fn test_non_sequential_burn_from() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);

        e.as_contract(&address, || {
            let token_id = 42;
            non_sequential_mint::<MockContractNonSequential>(&e, &owner, token_id);
            approve::<MockContractNonSequential>(&e, &owner, &spender, token_id, 1000);
            burn_from::<MockContractNonSequential>(&e, &spender, &owner, token_id);
            assert_eq!(total_supply::<MockContractNonSequential>(&e), 0);
        });
    }

    #[test]
    fn test_decrement_total_supply() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());

        e.as_contract(&address, || {
            increment_total_supply::<MockContractNonSequential>(&e);
            let initial_supply = total_supply::<MockContractNonSequential>(&e);
            decrement_total_supply::<MockContractNonSequential>(&e);
            assert_eq!(total_supply::<MockContractNonSequential>(&e), initial_supply - 1);
        });
    }

    #[test]
    fn test_add_to_global_enumeration() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());

        e.as_contract(&address, || {
            let token_id = 42;
            let total_supply = increment_total_supply::<MockContractNonSequential>(&e);
            add_to_global_enumeration::<MockContractNonSequential>(&e, token_id, total_supply);
            assert_eq!(get_token_id::<MockContractNonSequential>(&e, 0), token_id);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #308)")]
    fn test_remove_from_global_enumeration() {
        let e = Env::default();
        e.mock_all_auths();
        let address = e.register(MockContractNonSequential, ());

        e.as_contract(&address, || {
            let token_id = 42;
            let total_supply = increment_total_supply::<MockContractNonSequential>(&e);
            add_to_global_enumeration::<MockContractNonSequential>(&e, token_id, total_supply);
            remove_from_global_enumeration::<MockContractNonSequential>(&e, token_id, total_supply);

            get_token_id::<MockContractNonSequential>(&e, 0);
        });
    }
}
