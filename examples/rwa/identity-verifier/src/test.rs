extern crate std;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, map, panic_with_error, symbol_short,
    testutils::Address as _, vec, Address, Bytes, BytesN, Env, Map, String, Symbol, Vec,
};

const STORED_ID: Symbol = symbol_short!("stored_id");
const ISSUERS: Symbol = symbol_short!("issuers");
const CLAIM_IDS: Symbol = symbol_short!("claim_ids");
const CLAIM_OK: Symbol = symbol_short!("claim_ok");

use stellar_tokens::rwa::{
    claim_issuer::ClaimIssuer,
    identity_claims::{generate_claim_id, Claim},
};

use crate::contract::{IdentityVerifierContract, IdentityVerifierContractClient};

// ################## CLIENT FACTORY ##################

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
    identity_registry_storage: &Address,
    claim_topics_and_issuers: &Address,
) -> IdentityVerifierContractClient<'a> {
    let address = e.register(
        IdentityVerifierContract,
        (admin, manager, identity_registry_storage, claim_topics_and_issuers),
    );
    IdentityVerifierContractClient::new(e, &address)
}

// ################## MOCK CONTRACTS ##################

#[contract]
pub struct MockIdentityRegistryStorage;

#[contractimpl]
impl MockIdentityRegistryStorage {
    /// Returns the onchain identity address stored for the given account.
    pub fn stored_identity(e: &Env, _account: Address) -> Address {
        e.storage().persistent().get(&STORED_ID).unwrap()
    }

    /// Returns the recovery target for an old account, if one has been set.
    pub fn get_recovered_to(e: &Env, old_account: Address) -> Option<Address> {
        e.storage().persistent().get(&old_account)
    }
}

#[contract]
pub struct MockClaimTopicsAndIssuers;

#[contractimpl]
impl MockClaimTopicsAndIssuers {
    /// Returns a map of claim topic → list of trusted issuers.
    pub fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
        let issuers: Vec<Address> = e.storage().persistent().get(&ISSUERS).unwrap();
        map![e, (1u32, issuers)]
    }
}

#[contracttype]
pub enum IdentityClaimsMockKey {
    Claim(BytesN<32>),
}

#[contract]
pub struct MockIdentityClaims;

#[contractimpl]
impl MockIdentityClaims {
    /// Returns the claim for the given claim ID.
    pub fn get_claim(e: &Env, claim_id: BytesN<32>) -> Claim {
        e.storage().persistent().get(&IdentityClaimsMockKey::Claim(claim_id)).unwrap()
    }

    /// Returns all claim IDs for the given topic.
    pub fn get_claim_ids_by_topic(e: &Env, _topic: u32) -> Vec<BytesN<32>> {
        e.storage().persistent().get(&CLAIM_IDS).unwrap()
    }
}

#[contracterror]
pub enum MockClaimIssuerError {
    Invalid = 1,
}

#[contract]
pub struct MockClaimIssuer;

#[contractimpl]
impl ClaimIssuer for MockClaimIssuer {
    fn is_claim_valid(
        e: &Env,
        _identity: Address,
        _claim_topic: u32,
        _scheme: u32,
        _sig_data: Bytes,
        _claim_data: Bytes,
    ) {
        if !e.storage().persistent().get(&CLAIM_OK).unwrap_or(false) {
            panic_with_error!(e, MockClaimIssuerError::Invalid);
        }
    }
}

// ################## TEST HELPERS ##################

fn make_claim(e: &Env, issuer: &Address, topic: u32) -> Claim {
    Claim {
        topic,
        scheme: 1u32,
        issuer: issuer.clone(),
        signature: Bytes::from_array(e, &[1, 2, 3, 4]),
        data: Bytes::from_array(e, &[5, 6, 7, 8]),
        uri: String::from_str(e, "https://example.com"),
    }
}

/// Registers mock IRS, CTI, identity claims, and claim issuer contracts,
/// pre-wired with a single valid claim for topic 1 from `issuer`.
/// Returns `(identity_claims_addr, issuer_addr, irs_addr, cti_addr)`.
fn setup_identity_stack(e: &Env) -> (Address, Address, Address, Address) {
    let identity_claims = e.register(MockIdentityClaims, ());
    let issuer = e.register(MockClaimIssuer, ());
    let irs = e.register(MockIdentityRegistryStorage, ());
    let cti = e.register(MockClaimTopicsAndIssuers, ());

    // IRS: account → identity_claims contract
    e.as_contract(&irs, || {
        e.storage().persistent().set(&STORED_ID, &identity_claims);
    });

    // Identity claims: store one claim from `issuer` for topic 1
    e.as_contract(&identity_claims, || {
        let claim = make_claim(e, &issuer, 1);
        let claim_id = generate_claim_id(e, &issuer, 1);
        e.storage().persistent().set(&IdentityClaimsMockKey::Claim(claim_id.clone()), &claim);
        e.storage().persistent().set(&CLAIM_IDS, &Vec::from_array(e, [claim_id]));
    });

    // CTI: topic 1 → [issuer]
    e.as_contract(&cti, || {
        e.storage().persistent().set(&ISSUERS, &vec![e, issuer.clone()]);
    });

    (identity_claims, issuer, irs, cti)
}

// ################## CONSTRUCTOR / CONFIG ##################

#[test]
fn set_and_get_claim_topics_and_issuers_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let initial_irs = Address::generate(&e);
    let initial_cti = Address::generate(&e);
    let client = create_client(&e, &admin, &manager, &initial_irs, &initial_cti);
    let new_cti = Address::generate(&e);

    client.set_claim_topics_and_issuers(&new_cti, &manager);
    assert_eq!(client.claim_topics_and_issuers(), new_cti);
}

#[test]
fn set_and_get_identity_registry_storage_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let initial_irs = Address::generate(&e);
    let initial_cti = Address::generate(&e);
    let client = create_client(&e, &admin, &manager, &initial_irs, &initial_cti);
    let new_irs = Address::generate(&e);

    client.set_identity_registry_storage(&new_irs, &manager);
    assert_eq!(client.identity_registry_storage(), new_irs);
}

// ################## VERIFY IDENTITY ##################

#[test]
fn verify_identity_succeeds_with_valid_claim() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let account = Address::generate(&e);

    let (_identity, issuer, irs, cti) = setup_identity_stack(&e);

    e.as_contract(&issuer, || {
        e.storage().persistent().set(&CLAIM_OK, &true);
    });

    let client = create_client(&e, &admin, &manager, &irs, &cti);

    // Must not panic
    client.verify_identity(&account);
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn verify_identity_fails_with_invalid_claim() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let account = Address::generate(&e);

    let (_identity, issuer, irs, cti) = setup_identity_stack(&e);

    // Claim issuer rejects the claim
    e.as_contract(&issuer, || {
        e.storage().persistent().set(&CLAIM_OK, &false);
    });

    let client = create_client(&e, &admin, &manager, &irs, &cti);

    client.verify_identity(&account);
}

#[test]
fn verify_identity_succeeds_with_second_valid_issuer() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let account = Address::generate(&e);

    let (identity, issuer1, irs, cti) = setup_identity_stack(&e);
    let issuer2 = e.register(MockClaimIssuer, ());

    // issuer1 rejects, issuer2 accepts
    e.as_contract(&issuer1, || {
        e.storage().persistent().set(&CLAIM_OK, &false);
    });
    e.as_contract(&issuer2, || {
        e.storage().persistent().set(&CLAIM_OK, &true);
    });

    // Add claims for both issuers in identity contract
    e.as_contract(&identity, || {
        let id1 = generate_claim_id(&e, &issuer1, 1);
        let id2 = generate_claim_id(&e, &issuer2, 1);
        e.storage()
            .persistent()
            .set(&IdentityClaimsMockKey::Claim(id1.clone()), &make_claim(&e, &issuer1, 1));
        e.storage()
            .persistent()
            .set(&IdentityClaimsMockKey::Claim(id2.clone()), &make_claim(&e, &issuer2, 1));
        e.storage().persistent().set(&CLAIM_IDS, &Vec::from_array(&e, [id1, id2]));
    });

    // Register both issuers in CTI
    e.as_contract(&cti, || {
        e.storage().persistent().set(&ISSUERS, &vec![&e, issuer1.clone(), issuer2.clone()]);
    });

    let client = create_client(&e, &admin, &manager, &irs, &cti);

    client.verify_identity(&account);
}

// ################## RECOVERY TARGET ##################

#[test]
fn recovery_target_returns_none_when_not_set() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let old_account = Address::generate(&e);

    let (_identity, _issuer, irs, cti) = setup_identity_stack(&e);
    let client = create_client(&e, &admin, &manager, &irs, &cti);

    assert!(client.recovery_target(&old_account).is_none());
}

#[test]
fn recovery_target_returns_new_account_when_set() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let old_account = Address::generate(&e);
    let new_account = Address::generate(&e);

    let (_identity, _issuer, irs, cti) = setup_identity_stack(&e);

    // Register the recovery mapping in the IRS
    e.as_contract(&irs, || {
        e.storage().persistent().set(&old_account, &new_account);
    });

    let client = create_client(&e, &admin, &manager, &irs, &cti);

    assert_eq!(client.recovery_target(&old_account), Some(new_account));
}
