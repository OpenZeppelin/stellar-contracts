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
        e: Env,
        _lost_wallet: Address,
        _new_wallet: Address,
        investor_id: Address,
    ) -> bool {
        // block specific investor for mock
        if investor_id.to_string()
            == String::from_str(&e, "GC65CUPW2IMTJJY6CII7F3OBPVG4YGASEPBBLM4V3LBKX62P6LA24OFV")
        {
            return false;
        }
        true
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
fn set_and_get_claim_topics_and_issuers() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        let claim_topics_and_issuers = Address::generate(&e);
        RWA::set_claim_topics_and_issuers(&e, &claim_topics_and_issuers);
        assert_eq!(RWA::claim_topics_and_issuers(&e), claim_topics_and_issuers);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #311)")]
fn get_unset_claim_topics_and_issuers_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::claim_topics_and_issuers(&e);
    });
}

#[test]
fn set_and_get_identity_registry_storage() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        let identity_registry_storage = Address::generate(&e);
        RWA::set_identity_registry_storage(&e, &identity_registry_storage);
        assert_eq!(RWA::identity_registry_storage(&e), identity_registry_storage);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #312)")]
fn get_unset_identity_registry_storage_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::identity_registry_storage(&e);
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

#[test]
#[should_panic(expected = "Error(Contract, #305)")]
fn mint_without_compliance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let verifier = create_mock_identity_verifier(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::mint(&e, &to, 100);
    });
}

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
fn negative_amount_mint_fails() {
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

#[test]
fn contract_overrides_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Test ContractOverrides::transfer calls RWA::transfer
        RWA::transfer(&e, &from, &to, 30);

        assert_eq!(RWA::balance(&e, &from), 70);
        assert_eq!(RWA::balance(&e, &to), 30);
    });
}

#[test]
fn contract_overrides_transfer_from() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &owner, 100);

        // Set allowance
        RWA::approve(&e, &owner, &spender, 50, 1000);
        assert_eq!(RWA::allowance(&e, &owner, &spender), 50);
    });

    e.as_contract(&address, || {
        RWA::transfer_from(&e, &spender, &owner, &to, 30);

        assert_eq!(RWA::balance(&e, &owner), 70);
        assert_eq!(RWA::balance(&e, &to), 30);
        assert_eq!(RWA::allowance(&e, &owner, &spender), 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn forced_transfer_insufficient_balance_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 50);

        // Try to force transfer more than balance - should fail with
        // InsufficientBalance
        RWA::forced_transfer(&e, &from, &to, 100);
    });
}

#[test]
fn forced_transfer_with_token_unfreezing() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Freeze 60 tokens, leaving 40 free
        RWA::freeze_partial_tokens(&e, &from, 60);
        assert_eq!(RWA::get_frozen_tokens(&e, &from), 60);
        assert_eq!(RWA::get_free_tokens(&e, &from), 40);

        // Force transfer 70 tokens (more than free tokens)
        // This should automatically unfreeze 30 tokens (70 - 40)
        RWA::forced_transfer(&e, &from, &to, 70);

        // Verify balances
        assert_eq!(RWA::balance(&e, &from), 30);
        assert_eq!(RWA::balance(&e, &to), 70);

        // Verify frozen tokens were reduced by 30 (60 - 30 = 30)
        assert_eq!(RWA::get_frozen_tokens(&e, &from), 30);
        assert_eq!(RWA::get_free_tokens(&e, &from), 0); // 30 balance - 30
                                                        // frozen = 0 free
    });
}

#[test]
fn forced_transfer_without_unfreezing_needed() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Freeze 30 tokens, leaving 70 free
        RWA::freeze_partial_tokens(&e, &from, 30);
        assert_eq!(RWA::get_frozen_tokens(&e, &from), 30);
        assert_eq!(RWA::get_free_tokens(&e, &from), 70);

        // Force transfer 50 tokens (less than free tokens)
        // This should NOT unfreeze any tokens
        RWA::forced_transfer(&e, &from, &to, 50);

        // Verify balances
        assert_eq!(RWA::balance(&e, &from), 50);
        assert_eq!(RWA::balance(&e, &to), 50);

        // Verify frozen tokens remain unchanged
        assert_eq!(RWA::get_frozen_tokens(&e, &from), 30);
        assert_eq!(RWA::get_free_tokens(&e, &from), 20); // 50 balance - 30
                                                         // frozen = 20 free
    });
}

#[test]
fn forced_transfer_exact_unfreezing() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Freeze all tokens
        RWA::freeze_partial_tokens(&e, &from, 100);
        assert_eq!(RWA::get_frozen_tokens(&e, &from), 100);
        assert_eq!(RWA::get_free_tokens(&e, &from), 0);

        // Force transfer all tokens - should unfreeze all
        RWA::forced_transfer(&e, &from, &to, 100);

        // Verify balances
        assert_eq!(RWA::balance(&e, &from), 0);
        assert_eq!(RWA::balance(&e, &to), 100);

        // Verify all tokens were unfrozen
        assert_eq!(RWA::get_frozen_tokens(&e, &from), 0);
        assert_eq!(RWA::get_free_tokens(&e, &from), 0);
    });
}

#[test]
fn recovery_address_with_frozen_tokens() {
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

        // Mint tokens and freeze some
        RWA::mint(&e, &lost_wallet, 100);
        RWA::freeze_partial_tokens(&e, &lost_wallet, 60);
        assert_eq!(RWA::get_frozen_tokens(&e, &lost_wallet), 60);

        // Perform recovery
        let success = RWA::recovery_address(&e, &lost_wallet, &new_wallet, &investor_id);
        assert!(success);

        // Verify tokens were transferred and frozen tokens cleared
        assert_eq!(RWA::balance(&e, &lost_wallet), 0);
        assert_eq!(RWA::balance(&e, &new_wallet), 100);
        assert_eq!(RWA::get_frozen_tokens(&e, &lost_wallet), 0);
    });
}

#[test]
fn recovery_address_with_frozen_address() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        // Mint tokens and freeze the address
        RWA::mint(&e, &lost_wallet, 100);
        RWA::set_address_frozen(&e, &caller, &lost_wallet, true);
        assert!(RWA::is_frozen(&e, &lost_wallet));

        // Perform recovery
        let success = RWA::recovery_address(&e, &lost_wallet, &new_wallet, &investor_id);
        assert!(success);

        // Verify tokens were transferred and address unfrozen
        assert_eq!(RWA::balance(&e, &lost_wallet), 0);
        assert_eq!(RWA::balance(&e, &new_wallet), 100);
        assert!(!RWA::is_frozen(&e, &lost_wallet));
    });
}

#[test]
fn recovery_address_with_both_frozen_tokens_and_address() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        // Mint tokens, freeze some tokens, and freeze the address
        RWA::mint(&e, &lost_wallet, 100);
        RWA::freeze_partial_tokens(&e, &lost_wallet, 80);
        RWA::set_address_frozen(&e, &caller, &lost_wallet, true);

        assert_eq!(RWA::get_frozen_tokens(&e, &lost_wallet), 80);
        assert!(RWA::is_frozen(&e, &lost_wallet));

        // Perform recovery
        let success = RWA::recovery_address(&e, &lost_wallet, &new_wallet, &investor_id);
        assert!(success);

        // Verify tokens were transferred and both frozen status cleared
        assert_eq!(RWA::balance(&e, &lost_wallet), 0);
        assert_eq!(RWA::balance(&e, &new_wallet), 100);
        assert_eq!(RWA::get_frozen_tokens(&e, &lost_wallet), 0);
        assert!(!RWA::is_frozen(&e, &lost_wallet));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn freeze_partial_tokens_negative_amount_fails() {
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

        // Try to freeze negative amount - should fail
        RWA::freeze_partial_tokens(&e, &user, -10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn unfreeze_partial_tokens_negative_amount_fails() {
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
        RWA::freeze_partial_tokens(&e, &user, 50);

        // Try to unfreeze negative amount - should fail
        RWA::unfreeze_partial_tokens(&e, &user, -10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn transfer_fails_when_from_address_frozen() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Freeze the from address
        RWA::set_address_frozen(&e, &caller, &from, true);

        // Try to transfer - should fail with AddressFrozen error
        RWA::transfer(&e, &from, &to, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn transfer_fails_when_to_address_frozen() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Freeze the to address
        RWA::set_address_frozen(&e, &caller, &to, true);

        // Try to transfer - should fail with AddressFrozen error
        RWA::transfer(&e, &from, &to, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #303)")]
fn transfer_fails_when_insufficient_free_tokens() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        // SETUP
        let verifier = create_mock_identity_verifier(&e);
        let compliance = create_mock_compliance(&e);
        RWA::set_identity_verifier(&e, &verifier);
        RWA::set_compliance(&e, &compliance);

        RWA::mint(&e, &from, 100);

        // Freeze 80 tokens, leaving only 20 free
        RWA::freeze_partial_tokens(&e, &from, 80);
        assert_eq!(RWA::get_free_tokens(&e, &from), 20);

        // Try to transfer 50 tokens (more than free) - should fail
        RWA::transfer(&e, &from, &to, 50);
    });
}
