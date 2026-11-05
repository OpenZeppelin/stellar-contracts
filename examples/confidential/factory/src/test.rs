extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    xdr::{ScErrorCode, ScErrorType, ToXdr},
    Address, Bytes, BytesN, Env, Error, Symbol,
};
use stellar_confidential::{
    auditor::{storage as auditor_storage, ConfidentialAuditor, ConfidentialAuditorClient},
    compliance::{ComplianceConfig, ComplianceError},
    verifier::{CircuitType, ConfidentialVerifier},
    RegisterData, RegisterPayload,
};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::contract::{ConfidentialFactoryContract, ConfidentialFactoryContractClient};

mod token {
    soroban_sdk::contractimport!(file = "testdata/confidential_compliant_token_example.wasm");
}

mod allowlist {
    soroban_sdk::contractimport!(file = "testdata/confidential_allowlist_policy_example.wasm");
}

const AUDITOR_ID: u32 = 0;

#[contract]
struct MockVerifier;

#[contractimpl(contracttrait)]
impl ConfidentialVerifier for MockVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(_e: &Env, _ct: CircuitType, _pi: Bytes, _proof: Bytes) -> bool {
        true
    }
}

#[contract]
struct MockAuditor;

#[contractimpl(contracttrait)]
impl ConfidentialAuditor for MockAuditor {
    fn register_key(e: &Env, auditor_id: u32, point: BytesN<64>, _operator: Address) {
        auditor_storage::register_key(e, auditor_id, &point);
    }

    fn rotate_key(_e: &Env, _auditor_id: u32, _new_point: BytesN<64>, _operator: Address) {}
}

struct Harness<'a> {
    e: Env,
    factory: ConfidentialFactoryContractClient<'a>,
    underlying_asset: Address,
    verifier: Address,
    auditor: Address,
}

fn setup<'a>() -> Harness<'a> {
    let e = Env::default();
    e.mock_all_auths();

    let token_wasm_hash = e.deployer().upload_contract_wasm(token::WASM);
    let allowlist_wasm_hash = e.deployer().upload_contract_wasm(allowlist::WASM);
    let factory = e.register(ConfidentialFactoryContract, (token_wasm_hash, allowlist_wasm_hash));

    let underlying_asset = e.register_stellar_asset_contract_v2(Address::generate(&e)).address();
    let verifier = e.register(MockVerifier, ());
    let auditor = e.register(MockAuditor, ());
    ConfidentialAuditorClient::new(&e, &auditor).register_key(
        &AUDITOR_ID,
        &Grumpkin::generator(&e),
        &Address::generate(&e),
    );

    Harness {
        factory: ConfidentialFactoryContractClient::new(&e, &factory),
        underlying_asset,
        verifier,
        auditor,
        e,
    }
}

impl Harness<'_> {
    fn predict_token(&self, admin: &Address, compliance: &ComplianceConfig, salt: u32) -> Address {
        self.factory.predict_token_address(
            admin,
            &self.underlying_asset,
            &self.verifier,
            &self.auditor,
            compliance,
            &salt,
        )
    }

    fn deploy_token(&self, admin: &Address, compliance: &ComplianceConfig, salt: u32) -> Address {
        self.factory.deploy_token(
            admin,
            &self.underlying_asset,
            &self.verifier,
            &self.auditor,
            compliance,
            &salt,
        )
    }

    fn deploy_allowlist_and_token(
        &self,
        policy_admin: &Address,
        policy_manager: &Address,
        token_admin: &Address,
        sac_passthrough: bool,
        salt: u32,
    ) -> (Address, Address) {
        self.factory.deploy_allowlist_and_token(
            policy_admin,
            policy_manager,
            token_admin,
            &self.underlying_asset,
            &self.verifier,
            &self.auditor,
            &sac_passthrough,
            &salt,
        )
    }
}

fn no_policy() -> ComplianceConfig {
    ComplianceConfig { policy: None, sac_passthrough: false }
}

fn register_data(e: &Env) -> Bytes {
    RegisterData {
        payload: RegisterPayload { y: Grumpkin::generator(e), pvk: Grumpkin::generator(e) },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn existing_contract() -> Error {
    Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction)
}

#[test]
fn deploy_token_returns_the_predicted_address() {
    let h = setup();
    let admin = Address::generate(&h.e);
    let policy = Address::generate(&h.e);
    let compliance = ComplianceConfig { policy: Some(policy.clone()), sac_passthrough: true };

    let predicted = h.predict_token(&admin, &compliance, 0);
    let deployed = h.deploy_token(&admin, &compliance, 0);

    assert_eq!(deployed, predicted);
    let token = token::Client::new(&h.e, &deployed);
    assert_eq!(token.get_admin(), Some(admin));
    assert_eq!(
        token.compliance_config(),
        Some(token::ComplianceConfig { policy: Some(policy), sac_passthrough: true })
    );
}

#[test]
fn deploy_allowlist_returns_the_predicted_address() {
    let h = setup();
    let admin = Address::generate(&h.e);
    let manager = Address::generate(&h.e);

    let predicted = h.factory.predict_allowlist_address(&admin, &manager, &0);
    let deployed = h.factory.deploy_allowlist(&admin, &manager, &0);

    assert_eq!(deployed, predicted);
    let allowlist = allowlist::Client::new(&h.e, &deployed);
    assert_eq!(allowlist.get_admin(), Some(admin));
    assert!(allowlist.has_role(&manager, &Symbol::new(&h.e, "manager")).is_some());
}

#[test]
fn deploy_allowlist_and_token_returns_the_predicted_addresses() {
    let h = setup();
    let policy_admin = Address::generate(&h.e);
    let policy_manager = Address::generate(&h.e);
    let token_admin = Address::generate(&h.e);

    let (policy, token) =
        h.deploy_allowlist_and_token(&policy_admin, &policy_manager, &token_admin, true, 7);

    assert_eq!(policy, h.factory.predict_allowlist_address(&policy_admin, &policy_manager, &7));
    let compliance = ComplianceConfig { policy: Some(policy.clone()), sac_passthrough: true };
    assert_eq!(token, h.predict_token(&token_admin, &compliance, 7));
    assert_eq!(
        token::Client::new(&h.e, &token).compliance_config(),
        Some(token::ComplianceConfig { policy: Some(policy), sac_passthrough: true })
    );
}

#[test]
fn token_is_gated_by_the_allowlist_deployed_alongside_it() {
    let h = setup();
    let policy_admin = Address::generate(&h.e);
    let policy_manager = Address::generate(&h.e);
    let token_admin = Address::generate(&h.e);
    let alice = Address::generate(&h.e);

    let (policy, token) =
        h.deploy_allowlist_and_token(&policy_admin, &policy_manager, &token_admin, false, 0);
    let token = token::Client::new(&h.e, &token);

    assert_eq!(
        token.try_register(&alice, &AUDITOR_ID, &register_data(&h.e)),
        Err(Ok(ComplianceError::NotAuthorizedByPolicy.into()))
    );

    allowlist::Client::new(&h.e, &policy).allow(&alice, &policy_manager);
    token.register(&alice, &AUDITOR_ID, &register_data(&h.e));
    assert_eq!(token.confidential_balance(&alice).auditor_id, AUDITOR_ID);
}

#[test]
fn repeating_a_deploy_fails() {
    let h = setup();
    let admin = Address::generate(&h.e);
    let manager = Address::generate(&h.e);

    h.deploy_token(&admin, &no_policy(), 0);
    assert_eq!(
        h.factory.try_deploy_token(
            &admin,
            &h.underlying_asset,
            &h.verifier,
            &h.auditor,
            &no_policy(),
            &0
        ),
        Err(Ok(existing_contract()))
    );

    h.factory.deploy_allowlist(&admin, &manager, &0);
    assert_eq!(h.factory.try_deploy_allowlist(&admin, &manager, &0), Err(Ok(existing_contract())));

    h.deploy_allowlist_and_token(&admin, &manager, &admin, false, 1);
    assert_eq!(
        h.factory.try_deploy_allowlist_and_token(
            &admin,
            &manager,
            &admin,
            &h.underlying_asset,
            &h.verifier,
            &h.auditor,
            &false,
            &1
        ),
        Err(Ok(existing_contract()))
    );
}

#[test]
fn a_different_salt_yields_a_new_address() {
    let h = setup();
    let admin = Address::generate(&h.e);
    let manager = Address::generate(&h.e);

    let first_token = h.deploy_token(&admin, &no_policy(), 0);
    let second_token = h.deploy_token(&admin, &no_policy(), 1);
    assert_ne!(first_token, second_token);

    let first_allowlist = h.factory.deploy_allowlist(&admin, &manager, &0);
    let second_allowlist = h.factory.deploy_allowlist(&admin, &manager, &1);
    assert_ne!(first_allowlist, second_allowlist);
}

#[test]
fn every_argument_changes_the_address() {
    let h = setup();
    let a = Address::generate(&h.e);
    let b = Address::generate(&h.e);
    let with_policy = ComplianceConfig { policy: Some(a.clone()), sac_passthrough: false };
    let with_sac = ComplianceConfig { policy: None, sac_passthrough: true };
    let predict_token = |admin: &Address,
                         underlying_asset: &Address,
                         verifier: &Address,
                         auditor: &Address,
                         compliance: &ComplianceConfig,
                         salt: u32| {
        h.factory.predict_token_address(
            admin,
            underlying_asset,
            verifier,
            auditor,
            compliance,
            &salt,
        )
    };
    let (u, v, au) = (&h.underlying_asset, &h.verifier, &h.auditor);

    let addresses = [
        predict_token(&a, u, v, au, &no_policy(), 0),
        predict_token(&b, u, v, au, &no_policy(), 0),
        predict_token(&a, &b, v, au, &no_policy(), 0),
        predict_token(&a, u, &b, au, &no_policy(), 0),
        predict_token(&a, u, v, &b, &no_policy(), 0),
        predict_token(&a, u, v, au, &with_policy, 0),
        predict_token(&a, u, v, au, &with_sac, 0),
        predict_token(&a, u, v, au, &no_policy(), 1),
        h.factory.predict_allowlist_address(&a, &b, &0),
        h.factory.predict_allowlist_address(&b, &a, &0),
        h.factory.predict_allowlist_address(&a, &a, &0),
        h.factory.predict_allowlist_address(&a, &b, &1),
    ];

    for (i, address) in addresses.iter().enumerate() {
        for other in &addresses[i + 1..] {
            assert_ne!(address, other);
        }
    }
}
