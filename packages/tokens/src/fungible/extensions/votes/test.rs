extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, MuxedAddress};
use stellar_governance::votes::{delegate, get_delegate, get_votes, get_voting_units};

use crate::fungible::{extensions::votes::FungibleVotes, Base};

#[contract]
struct MockContract;

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
}

#[test]
fn mint_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);

        assert_eq!(Base::balance(&e, &alice), 100);
        assert_eq!(Base::total_supply(&e), 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
}

#[test]
fn mint_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        FungibleVotes::mint(&e, &alice, 100);

        assert_eq!(Base::balance(&e, &alice), 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
        assert_eq!(get_votes(&e, &bob), 100);
    });
}

#[test]
fn burn_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        FungibleVotes::burn(&e, &alice, 30);

        assert_eq!(Base::balance(&e, &alice), 70);
        assert_eq!(Base::total_supply(&e), 70);
        assert_eq!(get_voting_units(&e, &alice), 70);
    });
}

#[test]
fn burn_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        FungibleVotes::mint(&e, &alice, 100);
        assert_eq!(get_votes(&e, &bob), 100);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::burn(&e, &alice, 40);
        assert_eq!(get_votes(&e, &bob), 60);
    });
}

#[test]
fn burn_from_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 50, 1000);
        FungibleVotes::burn_from(&e, &spender, &owner, 30);

        assert_eq!(Base::balance(&e, &owner), 70);
        assert_eq!(get_voting_units(&e, &owner), 70);
    });
}

#[test]
fn transfer_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        FungibleVotes::transfer(&e, &alice, &MuxedAddress::from(bob.clone()), 40);

        assert_eq!(Base::balance(&e, &alice), 60);
        assert_eq!(Base::balance(&e, &bob), 40);
        assert_eq!(get_voting_units(&e, &alice), 60);
        assert_eq!(get_voting_units(&e, &bob), 40);
    });
}

#[test]
fn transfer_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let delegate_a = Address::generate(&e);
    let delegate_b = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        delegate(&e, &alice, &delegate_a);
        assert_eq!(get_votes(&e, &delegate_a), 100);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &bob, 50);
        delegate(&e, &bob, &delegate_b);
        assert_eq!(get_votes(&e, &delegate_b), 50);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::transfer(&e, &alice, &MuxedAddress::from(bob.clone()), 30);

        assert_eq!(get_votes(&e, &delegate_a), 70);
        assert_eq!(get_votes(&e, &delegate_b), 80);
    });
}

#[test]
fn transfer_from_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 50, 1000);
        FungibleVotes::transfer_from(&e, &spender, &owner, &recipient, 30);

        assert_eq!(Base::balance(&e, &owner), 70);
        assert_eq!(Base::balance(&e, &recipient), 30);
        assert_eq!(get_voting_units(&e, &owner), 70);
        assert_eq!(get_voting_units(&e, &recipient), 30);
    });
}

#[test]
fn transfer_from_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let delegate_owner = Address::generate(&e);
    let delegate_recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &owner, 100);
        delegate(&e, &owner, &delegate_owner);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &recipient, &delegate_recipient);
    });

    e.as_contract(&contract_address, || {
        Base::approve(&e, &owner, &spender, 50, 1000);
        FungibleVotes::transfer_from(&e, &spender, &owner, &recipient, 30);

        assert_eq!(get_votes(&e, &delegate_owner), 70);
        assert_eq!(get_votes(&e, &delegate_recipient), 30);
    });
}

#[test]
fn zero_mint_is_noop() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        delegate(&e, &alice, &bob);
        assert_eq!(get_votes(&e, &bob), 100);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 0);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
}

#[test]
fn zero_burn_is_noop() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::burn(&e, &alice, 0);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
}

#[test]
fn zero_transfer_is_noop() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::transfer(&e, &alice, &MuxedAddress::from(bob.clone()), 0);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
}

#[test]
fn delegate_before_mint() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        assert_eq!(get_delegate(&e, &alice), Some(bob.clone()));
        assert_eq!(get_votes(&e, &bob), 0);

        FungibleVotes::mint(&e, &alice, 100);
        assert_eq!(get_votes(&e, &bob), 100);
    });
}

#[test]
fn self_delegation() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        delegate(&e, &alice, &alice);

        assert_eq!(get_delegate(&e, &alice), Some(alice.clone()));
        assert_eq!(get_votes(&e, &alice), 100);
    });
}

#[test]
fn change_delegate_after_mint() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        delegate(&e, &alice, &bob);
        assert_eq!(get_votes(&e, &bob), 100);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &charlie);
        assert_eq!(get_votes(&e, &bob), 0);
        assert_eq!(get_votes(&e, &charlie), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn burn_insufficient_balance_panics() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        FungibleVotes::burn(&e, &alice, 150);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn transfer_insufficient_balance_panics() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        FungibleVotes::transfer(&e, &alice, &MuxedAddress::from(bob.clone()), 150);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn transfer_from_insufficient_allowance_panics() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 20, 1000);
        FungibleVotes::transfer_from(&e, &spender, &owner, &recipient, 50);
    });
}

#[test]
fn multiple_holders_with_same_delegate() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        FungibleVotes::mint(&e, &bob, 50);

        delegate(&e, &alice, &charlie);
        delegate(&e, &bob, &charlie);

        assert_eq!(get_votes(&e, &charlie), 150);
    });
}

#[test]
fn transfer_between_delegated_accounts() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        FungibleVotes::mint(&e, &bob, 50);
        delegate(&e, &alice, &alice);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &bob, &bob);
    });

    e.as_contract(&contract_address, || {
        assert_eq!(get_votes(&e, &alice), 100);
        assert_eq!(get_votes(&e, &bob), 50);

        FungibleVotes::transfer(&e, &alice, &MuxedAddress::from(bob.clone()), 30);

        assert_eq!(get_votes(&e, &alice), 70);
        assert_eq!(get_votes(&e, &bob), 80);
    });
}

#[test]
fn burn_all_tokens() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        FungibleVotes::mint(&e, &alice, 100);
        delegate(&e, &alice, &bob);
        assert_eq!(get_votes(&e, &bob), 100);
    });

    e.as_contract(&contract_address, || {
        FungibleVotes::burn(&e, &alice, 100);

        assert_eq!(Base::balance(&e, &alice), 0);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(get_votes(&e, &bob), 0);
    });
}
