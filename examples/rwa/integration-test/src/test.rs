extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use stellar_tokens::rwa::identity_registry_storage::CountryData;

// Import all the separated contracts using contractimport from testdata
mod claim_issuer {
    soroban_sdk::contractimport!(file = "../testdata/claim_issuer_example.wasm");
}

mod claim_topics_and_issuers {
    soroban_sdk::contractimport!(file = "../testdata/claim_topics_and_issuers_example.wasm");
}

mod compliance {
    soroban_sdk::contractimport!(file = "../testdata/compliance_example.wasm");
}

mod identity_claims {
    soroban_sdk::contractimport!(file = "../testdata/identity_claims_example.wasm");
}

mod identity_registry_storage {
    soroban_sdk::contractimport!(file = "../testdata/identity_registry_storage_example.wasm");
}

mod rwa_token {
    soroban_sdk::contractimport!(file = "../testdata/rwa_token_example.wasm");
}

mod transfer_limit_module {
    soroban_sdk::contractimport!(file = "../testdata/transfer_limit_module_example.wasm");
}

mod country_restriction_module {
    soroban_sdk::contractimport!(file = "../testdata/country_restriction_module_example.wasm");
}

#[test]
fn test_full_rwa_integration() {
    let e = Env::default();
    e.mock_all_auths();

    // Generate addresses for different roles
    let admin = Address::generate(&e);
    let compliance_officer = Address::generate(&e);
    let recovery_agent = Address::generate(&e);
    let user = Address::generate(&e);
    let identity_contract = Address::generate(&e);

    // Deploy all contracts
    let claim_issuer_id = e.register(claim_issuer::WASM, (&admin,));
    let claim_issuer_client = claim_issuer::Client::new(&e, &claim_issuer_id);

    let claim_topics_id = e.register(claim_topics_and_issuers::WASM, (&admin,));
    let claim_topics_client = claim_topics_and_issuers::Client::new(&e, &claim_topics_id);

    let compliance_id = e.register(compliance::WASM, (&admin,));
    let compliance_client = compliance::Client::new(&e, &compliance_id);

    let identity_claims_id = e.register(identity_claims::WASM, (&admin,));
    let identity_claims_client = identity_claims::Client::new(&e, &identity_claims_id);

    let identity_registry_id = e.register(identity_registry_storage::WASM, (&admin,));
    let identity_registry_client =
        identity_registry_storage::Client::new(&e, &identity_registry_id);

    let transfer_limit_id = e.register(transfer_limit_module::WASM, (&admin,));
    let transfer_limit_client = transfer_limit_module::Client::new(&e, &transfer_limit_id);

    let country_restriction_id = e.register(country_restriction_module::WASM, (&admin,));
    let country_restriction_client =
        country_restriction_module::Client::new(&e, &country_restriction_id);

    let rwa_token_id = e.register(
        rwa_token::WASM,
        (
            &admin,
            &compliance_officer,
            &recovery_agent,
            &String::from_str(&e, "RWA Token"),
            &String::from_str(&e, "RWA"),
            &18u32,
        ),
    );
    let rwa_token_client = rwa_token::Client::new(&e, &rwa_token_id);

    // Step 1: Setup claim topics and issuers
    claim_topics_client.setup_default_topics(&admin);
    let topics = claim_topics_client.get_claim_topics();
    assert_eq!(topics.len(), 4); // KYC, AML, Accredited Investor, Country Verification

    // Add the claim issuer as trusted for KYC topic
    let kyc_topics = Vec::from_array(&e, [1u32]); // KYC topic
    claim_topics_client.add_trusted_issuer(&claim_issuer_id, &kyc_topics, &admin);
    assert!(claim_topics_client.is_trusted_issuer(&claim_issuer_id));

    // Step 2: Setup identity registry
    let country_data = CountryData {
        country: 840u16, // USA
        region: 1u8,
        text: String::from_str(&e, "United States"),
    };
    let country_data_list = Vec::from_array(&e, [country_data]);

    identity_registry_client.add_identity(&user, &identity_contract, &country_data_list, &admin);
    assert_eq!(identity_registry_client.stored_identity(&user), identity_contract);

    // Step 3: Setup compliance with modules
    use stellar_tokens::rwa::compliance::ComplianceHook;

    compliance_client.add_module_to(&ComplianceHook::CanTransfer, &transfer_limit_id, &admin);
    compliance_client.add_module_to(&ComplianceHook::CanTransfer, &country_restriction_id, &admin);

    let modules = compliance_client.get_modules_for_hook(&ComplianceHook::CanTransfer);
    assert_eq!(modules.len(), 2);

    // Step 4: Configure RWA token with all contracts
    rwa_token_client.set_compliance(&compliance_id, &admin);
    rwa_token_client.set_claim_topics_and_issuers(&claim_topics_id, &admin);
    rwa_token_client.set_identity_registry_storage(&identity_registry_id, &admin);

    // Verify configuration
    assert_eq!(rwa_token_client.compliance(), compliance_id);
    assert_eq!(rwa_token_client.claim_topics_and_issuers(), claim_topics_id);
    assert_eq!(rwa_token_client.identity_registry_storage(), identity_registry_id);

    // Step 5: Test token operations
    let mint_amount = 1000i128;

    // Mint tokens to user
    rwa_token_client.mint(&user, &mint_amount, &admin);
    assert_eq!(rwa_token_client.balance(&user), mint_amount);

    // Test freezing functionality
    assert!(!rwa_token_client.is_frozen(&user));
    rwa_token_client.set_address_frozen(&user, &true, &compliance_officer);
    assert!(rwa_token_client.is_frozen(&user));

    rwa_token_client.set_address_frozen(&user, &false, &compliance_officer);
    assert!(!rwa_token_client.is_frozen(&user));

    // Test partial token freezing
    let freeze_amount = 500i128;
    rwa_token_client.freeze_partial_tokens(&user, &freeze_amount, &compliance_officer);
    assert_eq!(rwa_token_client.get_frozen_tokens(&user), freeze_amount);

    // Step 6: Test compliance module functionality
    let another_user = Address::generate(&e);

    // Test transfer limits (should pass for reasonable amounts)
    assert!(transfer_limit_client.can_transfer(&user, &another_user, &mint_amount));

    // Test country restrictions (currently allows all)
    assert!(country_restriction_client.can_transfer(&user, &another_user, &mint_amount));

    // Step 7: Test claim issuer functionality
    let key = soroban_sdk::Bytes::from_array(&e, &[1, 2, 3, 4]);
    claim_issuer_client.allow_key(&key, &1u32, &admin); // Allow key for KYC topic
    assert!(claim_issuer_client.is_key_allowed(&key, &1u32));

    // Test claim revocation
    let claim_digest = soroban_sdk::BytesN::from_array(&e, &[0u8; 32]);
    assert!(!claim_issuer_client.is_claim_revoked(&claim_digest));
    claim_issuer_client.revoke_claim(&claim_digest, &admin);
    assert!(claim_issuer_client.is_claim_revoked(&claim_digest));
}

#[test]
fn test_compliance_integration() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    // Deploy compliance and modules
    let compliance_id = e.register(compliance::WASM, (&admin,));
    let compliance_client = compliance::Client::new(&e, &compliance_id);

    let transfer_limit_id = e.register(transfer_limit_module::WASM, (&admin,));
    let transfer_limit_client = transfer_limit_module::Client::new(&e, &transfer_limit_id);

    // Register transfer limit module with compliance
    use stellar_tokens::rwa::compliance::ComplianceHook;
    compliance_client.add_module_to(&ComplianceHook::CanTransfer, &transfer_limit_id, &admin);
    compliance_client.add_module_to(&ComplianceHook::CanCreate, &transfer_limit_id, &admin);

    // Test compliance checks
    let normal_amount = 500_000_000i128; // 500M tokens (within limit)
    let excessive_amount = 2_000_000_000i128; // 2B tokens (exceeds limit)

    // Normal transfer should be allowed
    assert!(compliance_client.can_transfer(&user1, &user2, &normal_amount));

    // Excessive transfer should be blocked
    assert!(!compliance_client.can_transfer(&user1, &user2, &excessive_amount));

    // Normal minting should be allowed
    assert!(compliance_client.can_create(&user1, &normal_amount));

    // Excessive minting should be blocked
    let excessive_mint = 15_000_000_000i128; // 15B tokens (exceeds limit)
    assert!(!compliance_client.can_create(&user1, &excessive_mint));

    // Test hook notifications (should not fail)
    compliance_client.transferred(&user1, &user2, &normal_amount);
    compliance_client.created(&user1, &normal_amount);
    compliance_client.destroyed(&user1, &normal_amount);
}

#[test]
fn test_identity_and_claims_integration() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let identity_contract = Address::generate(&e);
    let issuer = Address::generate(&e);

    // Deploy contracts
    let identity_registry_id = e.register(identity_registry_storage::WASM, (&admin,));
    let identity_registry_client =
        identity_registry_storage::Client::new(&e, &identity_registry_id);

    let identity_claims_id = e.register(identity_claims::WASM, (&admin,));
    let identity_claims_client = identity_claims::Client::new(&e, &identity_claims_id);

    let claim_topics_id = e.register(claim_topics_and_issuers::WASM, (&admin,));
    let claim_topics_client = claim_topics_and_issuers::Client::new(&e, &claim_topics_id);

    // Setup identity with country data
    let country_data = CountryData {
        country: 124u16, // Canada
        region: 1u8,
        text: String::from_str(&e, "Canada"),
    };
    let country_data_list = Vec::from_array(&e, [country_data.clone()]);

    identity_registry_client.add_identity(&user, &identity_contract, &country_data_list, &admin);

    // Verify identity storage
    assert_eq!(identity_registry_client.stored_identity(&user), identity_contract);
    let stored_country_data = identity_registry_client.get_country_data(&user, &0u32);
    assert_eq!(stored_country_data.country, country_data.country);

    // Setup claim topics
    claim_topics_client.add_claim_topic(&1u32, &admin); // KYC
    claim_topics_client.add_claim_topic(&2u32, &admin); // AML

    let topics = claim_topics_client.get_claim_topics();
    assert!(topics.contains(&1u32));
    assert!(topics.contains(&2u32));

    // Add trusted issuer
    let issuer_topics = Vec::from_array(&e, [1u32, 2u32]);
    claim_topics_client.add_trusted_issuer(&issuer, &issuer_topics, &admin);
    assert!(claim_topics_client.is_trusted_issuer(&issuer));
    assert!(claim_topics_client.has_claim_topic(&issuer, &1u32));

    // Add identity claims
    let signature = soroban_sdk::Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = soroban_sdk::Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com/kyc-claim");

    let claim_id = identity_claims_client.add_claim(&1u32, &1u32, &issuer, &signature, &data, &uri);

    // Verify claim storage
    let stored_claim = identity_claims_client.get_claim(&claim_id);
    assert_eq!(stored_claim.topic, 1u32);
    assert_eq!(stored_claim.issuer, issuer);

    // Get claims by topic
    let kyc_claims = identity_claims_client.get_claim_ids_by_topic(&1u32);
    assert_eq!(kyc_claims.len(), 1);
    assert_eq!(kyc_claims.get(0).unwrap(), claim_id);
}
