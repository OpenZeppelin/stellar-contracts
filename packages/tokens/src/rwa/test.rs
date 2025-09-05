extern crate std;

use soroban_sdk::{
    contract, contractimpl, map, symbol_short, testutils::Address as _, vec, Address, Bytes, Env,
    Map, String, Vec,
};

use super::{claim_issuer::ClaimIssuer, identity_claims::Claim};
use crate::{
    fungible::ContractOverrides,
    rwa::{storage::RWAStorageKey, RWA},
};

#[contract]
pub struct MockIdentityRegistryStorage;

#[contractimpl]
impl MockIdentityRegistryStorage {
    pub fn stored_identity(e: &Env, _account: Address) -> Address {
        e.storage().persistent().get(&symbol_short!("stored_id")).unwrap()
    }
}

#[contract]
pub struct MockClaimTopicsAndIssuers;

#[contractimpl]
impl MockClaimTopicsAndIssuers {
    pub fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
        let issuers = e.storage().persistent().get(&symbol_short!("issuers")).unwrap();
        map![e, (1u32, issuers)]
    }
}

#[contract]
pub struct MockIdentityClaims;

#[contractimpl]
impl MockIdentityClaims {
    pub fn get_claim(e: &Env, _claim_id: soroban_sdk::BytesN<32>) -> Claim {
        let default = Claim {
            topic: 1u32,
            scheme: 1u32,
            issuer: Address::generate(e),
            signature: Bytes::from_array(e, &[1, 2, 3, 4]),
            data: Bytes::from_array(e, &[5, 6, 7, 8]),
            uri: soroban_sdk::String::from_str(e, "https://example.com"),
        };
        e.storage().persistent().get(&symbol_short!("claim")).unwrap_or(default)
    }
}

#[contract]
pub struct MockClaimIssuer;

#[contractimpl]
impl ClaimIssuer for MockClaimIssuer {
    fn is_claim_valid(
        e: &Env,
        _identity: Address,
        _claim_topic: u32,
        _sig_data: Bytes,
        _claim_data: Bytes,
    ) -> bool {
        e.storage().persistent().get(&symbol_short!("claim_ok")).unwrap_or(true)
    }
}

// Mock Compliance Contract
#[contract]
struct MockCompliance;

#[contractimpl]
impl MockCompliance {
    pub fn can_transfer(e: Env, _from: Address, _to: Address, _amount: i128) -> bool {
        e.storage().persistent().get(&symbol_short!("tx_ok")).unwrap_or(true)
    }

    pub fn can_create(e: Env, _to: Address, _amount: i128) -> bool {
        e.storage().persistent().get(&symbol_short!("mint_ok")).unwrap_or(true)
    }

    pub fn transferred(_e: Env, _from: Address, _to: Address, _amount: i128) {}

    pub fn created(_e: Env, _to: Address, _amount: i128) {}

    pub fn destroyed(_e: Env, _from: Address, _amount: i128) {}
}

#[contract]
struct MockRWAContract;

fn construct_claim(e: &Env, issuer: &Address, topic: u32) -> Claim {
    Claim {
        topic,
        scheme: 1u32,
        issuer: issuer.clone(),
        signature: Bytes::from_array(e, &[1, 2, 3, 4]),
        data: Bytes::from_array(e, &[5, 6, 7, 8]),
        uri: soroban_sdk::String::from_str(e, "https://example.com"),
    }
}

fn set_and_return_verification_contracts(e: &Env) -> (Address, Address, Address, Address) {
    let identity = e.register(MockIdentityClaims, ());
    let issuer = e.register(MockClaimIssuer, ());
    let irs = e.register(MockIdentityRegistryStorage, ());
    let cti = e.register(MockClaimTopicsAndIssuers, ());

    e.as_contract(&irs, || {
        // Set up storage with mock contract addresses
        e.storage().persistent().set(&symbol_short!("stored_id"), &identity);
    });
    e.as_contract(&identity, || {
        let claim = construct_claim(e, &issuer, 1);
        e.storage().persistent().set(&symbol_short!("claim"), &claim);
    });

    e.as_contract(&cti, || {
        // Set up storage with mock contract addresses
        e.storage().persistent().set(&symbol_short!("issuers"), &vec![&e, issuer.clone()]);
    });

    RWA::set_claim_topics_and_issuers(e, &cti);
    RWA::set_identity_registry_storage(e, &irs);

    (identity, issuer, irs, cti)
}

fn set_and_return_compliance(e: &Env) -> Address {
    let compliance = e.register(MockCompliance, ());
    RWA::set_compliance(e, &compliance);
    compliance
}

fn setup_all_contracts(e: &Env) {
    let _ = set_and_return_verification_contracts(e);
    let _ = set_and_return_compliance(e);
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
#[should_panic(expected = "Error(Contract, #309)")]
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
#[should_panic(expected = "Error(Contract, #307)")]
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
#[should_panic(expected = "Error(Contract, #310)")]
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
#[should_panic(expected = "Error(Contract, #311)")]
fn get_unset_identity_registry_storage_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());

    e.as_contract(&address, || {
        RWA::identity_registry_storage(&e);
    });
}

#[test]
fn mint_tokens() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

        RWA::mint(&e, &to, 100);
        assert_eq!(RWA::balance(&e, &to), 100);
        assert_eq!(RWA::total_supply(&e), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn mint_fails_when_not_same_claim_topic() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let (identity, issuer, ..) = set_and_return_verification_contracts(&e);
        e.as_contract(&identity, || {
            let claim = construct_claim(&e, &issuer, 2);
            e.storage().persistent().set(&symbol_short!("claim"), &claim);
        });

        set_and_return_compliance(&e);

        RWA::mint(&e, &to, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn mint_fails_when_not_same_issuers() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let (identity, ..) = set_and_return_verification_contracts(&e);
        let other_issuer = e.register(MockClaimIssuer, ());
        e.as_contract(&identity, || {
            let claim = construct_claim(&e, &other_issuer, 1);
            e.storage().persistent().set(&symbol_short!("claim"), &claim);
        });

        set_and_return_compliance(&e);

        RWA::mint(&e, &to, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn mint_fails_when_claim_not_valid() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let (_, claim_issuer, ..) = set_and_return_verification_contracts(&e);
        e.as_contract(&claim_issuer, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &false)
        });

        set_and_return_compliance(&e);

        RWA::mint(&e, &to, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn mint_fails_with_claim_issuer_conversion_error() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let (_, claim_issuer, ..) = set_and_return_verification_contracts(&e);
        // Claim issuer returns invalid data, u32 instead of bool
        e.as_contract(&claim_issuer, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &12u32)
        });

        set_and_return_compliance(&e);

        RWA::mint(&e, &to, 100);
    });
}

#[test]
fn mint_with_two_claim_issuers() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        let (identity, claim_issuer, _, cti) = set_and_return_verification_contracts(&e);

        // First claim issuer returns invalid data, u32 instead of bool
        e.as_contract(&claim_issuer, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &12u32)
        });

        // Second claim issuer returns claim is valid
        let claim_issuer_2 = e.register(MockClaimIssuer, ());
        e.as_contract(&identity, || {
            let claim = construct_claim(&e, &claim_issuer_2, 1);
            e.storage().persistent().set(&symbol_short!("claim"), &claim);
        });
        e.as_contract(&cti, || {
            e.storage()
                .persistent()
                .set(&symbol_short!("issuers"), &vec![&e, claim_issuer.clone(), claim_issuer_2]);
        });

        set_and_return_compliance(&e);

        RWA::mint(&e, &to, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #307)")]
fn mint_without_compliance_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        set_and_return_verification_contracts(&e);
        RWA::mint(&e, &to, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #305)")]
fn mint_fails_when_not_compliant() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);

    let failing_compliance = e.register(MockCompliance, ());
    e.as_contract(&failing_compliance, || {
        e.storage().persistent().set(&symbol_short!("mint_ok"), &false);
    });

    e.as_contract(&address, || {
        set_and_return_verification_contracts(&e);
        RWA::set_compliance(&e, &failing_compliance);

        RWA::mint(&e, &from, 100);
    });
}

#[test]
fn burn_tokens() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        RWA::burn(&e, &account, 100);
    });
}

#[test]
fn forced_transfer() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

        RWA::mint(&e, &user, 50);
        RWA::freeze_partial_tokens(&e, &user, 100); // Should fail
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #303)")]
fn unfreeze_more_than_frozen_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

        RWA::mint(&e, &user, 100);
        RWA::freeze_partial_tokens(&e, &user, 30);
        RWA::unfreeze_partial_tokens(&e, &user, 50); // Should fail
    });
}

#[test]
fn recovery_address() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

        // No tokens in lost wallet
        let success = RWA::recovery_address(&e, &lost_wallet, &new_wallet, &investor_id);
        assert!(!success); // Should return false for zero balance
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn negative_amount_mint_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
        setup_all_contracts(&e);

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
        setup_all_contracts(&e);

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
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

        RWA::mint(&e, &from, 50);

        // Try to force transfer more than balance - should fail with
        // InsufficientBalance
        RWA::forced_transfer(&e, &from, &to, 100);
    });
}

#[test]
fn forced_transfer_with_token_unfreezing() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let lost_wallet = Address::generate(&e);
    let new_wallet = Address::generate(&e);
    let investor_id = Address::generate(&e);
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

        RWA::mint(&e, &user, 100);

        // Try to freeze negative amount - should fail
        RWA::freeze_partial_tokens(&e, &user, -10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn unfreeze_partial_tokens_negative_amount_fails() {
    let e = Env::default();
    let address = e.register(MockRWAContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        setup_all_contracts(&e);

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
        setup_all_contracts(&e);

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
        setup_all_contracts(&e);

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
        setup_all_contracts(&e);

        RWA::mint(&e, &from, 100);

        // Freeze 80 tokens, leaving only 20 free
        RWA::freeze_partial_tokens(&e, &from, 80);
        assert_eq!(RWA::get_free_tokens(&e, &from), 20);

        // Try to transfer 50 tokens (more than free) - should fail
        RWA::transfer(&e, &from, &to, 50);
    });
}
