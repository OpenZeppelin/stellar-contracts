#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};

use crate::{
    fungible::ContractOverrides,
    rwa::{storage::RWAStorageKey, RWA},
};

// Mock Identity Verifier Contract
#[contract]
struct MockIdentityVerifier;

#[contractimpl]
impl MockIdentityVerifier {
    pub fn is_verified(_e: Env, _address: Address) -> bool {
        true // Always return true for testing
    }

    pub fn can_recover(
        _e: Env,
        _lost_wallet: Address,
        _new_wallet: Address,
        _investor_id: Address,
    ) -> bool {
        true // Always return true for testing
    }
}

// Mock Compliance Contract
#[contract]
struct MockCompliance;

#[contractimpl]
impl MockCompliance {
    pub fn can_transfer(_e: Env, _from: Option<Address>, _to: Address, _amount: i128) -> bool {
        true // Always return true for testing
    }

    pub fn transferred(_e: Env, _from: Address, _to: Address, _amount: i128) {}

    pub fn created(_e: Env, _to: Address, _amount: i128) {}

    pub fn destroyed(_e: Env, _from: Address, _amount: i128) {}
}

#[contract]
struct MockRWAContract;

// Helper function to create a mock identity verifier contract
fn create_mock_identity_verifier(e: &Env) -> Address {
    e.register(MockIdentityVerifier, ())
}

// Helper function to create a mock compliance contract
fn create_mock_compliance(e: &Env) -> Address {
    e.register(MockCompliance, ())
}

#[test]
fn get_version() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        e.storage().instance().set(&RWAStorageKey::Version, &String::from_str(&e, "1,2,3"));
        assert_eq!(RWA::version(&e), String::from_str(&e, "1,2,3"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #310)")]
fn get_unset_version_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::version(&e);
    });
}

#[test]
fn set_and_get_onchain_id() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        let onchain_id = Address::generate(&e);
        RWA::set_onchain_id(&e, &onchain_id);
        assert_eq!(RWA::onchain_id(&e), onchain_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #308)")]
fn get_unset_onchain_id_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::onchain_id(&e);
    });
}

#[test]
fn set_and_get_identity_verifier() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        let verifier = Address::generate(&e);
        RWA::set_identity_verifier(&e, &verifier);
        assert_eq!(RWA::identity_verifier(&e), verifier);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn get_unset_identity_verifier_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::identity_verifier(&e);
    });
}

#[test]
fn set_and_get_compliance() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        let compliance = Address::generate(&e);
        RWA::set_compliance(&e, &compliance);
        assert_eq!(RWA::compliance(&e), compliance);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #305)")]
fn get_unset_compliance_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::compliance(&e);
    });
}

#[test]
fn mint_with_identity_verification() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &to, 100);
        assert_eq!(RWA::balance(&e, &to), 100);
        assert_eq!(RWA::total_supply(&e), 100);
    });
}

// TODO: mint without compliance should panic test

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn mint_without_identity_verifier_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        RWA::mint(&e, &to, 100);
    });
}

#[test]
fn burn_tokens() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &account, 100);
        assert_eq!(RWA::balance(&e, &account), 100);

        // Now burn tokens
        RWA::burn(&e, &account, 30);
        assert_eq!(RWA::balance(&e, &account), 70);
        assert_eq!(RWA::total_supply(&e), 70);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn burn_insufficient_balance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        RWA::burn(&e, &account, 100);
    });
}

#[test]
fn forced_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // mint tokens
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Perform forced transfer
        RWA::forced_transfer(&e, &from, &to, 50);
        assert_eq!(RWA::balance(&e, &from), 50);
        assert_eq!(RWA::balance(&e, &to), 50);
    });
}

#[test]
fn address_freezing() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let caller = Address::generate(&e);
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Initially not frozen
        assert!(!RWA::is_frozen(&e, &user));

        // Freeze the address
        RWA::set_address_frozen(&e, &caller, &user, true);
        assert!(RWA::is_frozen(&e, &user));

        // Unfreeze the address
        RWA::set_address_frozen(&e, &caller, &user, false);
        assert!(!RWA::is_frozen(&e, &user));
    });
}

#[test]
fn partial_token_freezing() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &user, 100);

        // Initially no frozen tokens
        assert_eq!(RWA::get_frozen_tokens(&e, &user), 0);

        // Freeze some tokens
        RWA::freeze_partial_tokens(&e, &user, 30);
        assert_eq!(RWA::get_frozen_tokens(&e, &user), 30);

        // Unfreeze some tokens
        RWA::unfreeze_partial_tokens(&e, &user, 10);
        assert_eq!(RWA::get_frozen_tokens(&e, &user), 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn freeze_more_than_balance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &user, 50);
        RWA::freeze_partial_tokens(&e, &user, 100); // Should fail
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #303)")]
fn unfreeze_more_than_frozen_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &user, 100);
        RWA::freeze_partial_tokens(&e, &user, 30);
        RWA::unfreeze_partial_tokens(&e, &user, 50); // Should fail
    });
}

#[test]
fn recovery_address() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        // Mint tokens to lost wallet
        RWA::mint(&e, &lost_wallet, 100);
        assert_eq!(RWA::balance(&e, &lost_wallet), 100);

        // Perform recovery
        let success = RWA::recovery_address(&e, &lost_wallet, &new_wallet, &investor_id);
        assert!(success);

        // Verify tokens were transferred
        assert_eq!(RWA::balance(&e, &lost_wallet), 0);
        assert_eq!(RWA::balance(&e, &new_wallet), 100);
    });
}

#[test]
fn recovery_with_zero_balance_returns_false() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);

    e.as_contract(&address, || {
        let verifier = create_mock_identity_verifier(&e);
        RWA::set_identity_verifier(&e, &verifier);

        // No tokens in lost wallet
        let success = RWA::recovery_address(&e, &lost_wallet, &new_wallet, &investor_id);
        assert!(!success); // Should return false for zero balance
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn negative_amount_operations_fail() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &user, -100);
    });
}

#[test]
fn transfer_with_compliance_and_identity_checks() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        // Mint and transfer
        RWA::mint(&e, &from, 100);
        RWA::transfer(&e, &from, &to, 50);

        assert_eq!(RWA::balance(&e, &from), 50);
        assert_eq!(RWA::balance(&e, &to), 50);
    });
}
