extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::Address as _,
    token::StellarAssetClient,
    xdr::{AccountFlags, ToXdr},
    Address, Bytes, BytesN, Env, Error, Symbol,
};
use stellar_access::access_control::AccessControlError;
use stellar_confidential::{
    auditor::{storage as auditor_storage, ConfidentialAuditor, ConfidentialAuditorClient},
    compliance::{ClawbackData, ComplianceConfig, ComplianceError, Policy},
    verifier::{CircuitType, ConfidentialVerifier},
    RegisterData, RegisterPayload, SetSpenderData, SetSpenderPayload, TransferData,
    TransferPayload, WithdrawData, WithdrawPayload,
};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::contract::{CompliantTokenContract, CompliantTokenContractClient};

const AUDITOR_ID: u32 = 0;

// ################## MOCK CONTRACTS ##################

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

#[contracttype]
enum MockAllowlistKey {
    Allowed(Address),
}

/// Stands in for `examples/confidential/allowlist-policy`, which is a
/// `cdylib` and cannot be linked into this test.
#[contract]
struct MockAllowlist;

#[contractimpl]
impl MockAllowlist {
    pub fn allow(e: &Env, account: Address) {
        e.storage().persistent().set(&MockAllowlistKey::Allowed(account), &());
    }

    pub fn disallow(e: &Env, account: Address) {
        e.storage().persistent().remove(&MockAllowlistKey::Allowed(account));
    }
}

#[contractimpl]
impl Policy for MockAllowlist {
    fn is_authorized(e: Env, account: Address, _token: Address) -> bool {
        e.storage().persistent().has(&MockAllowlistKey::Allowed(account))
    }
}

// ################## SETUP ##################

struct Harness<'a> {
    e: Env,
    sac: StellarAssetClient<'a>,
    verifier: Address,
    auditor: Address,
    admin: Address,
    freezer: Address,
    config_manager: Address,
    clawback: Address,
}

fn setup<'a>() -> Harness<'a> {
    let e = Env::default();
    e.mock_all_auths();

    let sac = e.register_stellar_asset_contract_v2(Address::generate(&e));
    // SAC v2 honors `set_authorized(_, false)` only on a revocable issuer.
    sac.issuer().set_flag(AccountFlags::RevocableFlag);
    let sac = StellarAssetClient::new(&e, &sac.address());

    let verifier = e.register(MockVerifier, ());
    let auditor = e.register(MockAuditor, ());
    ConfidentialAuditorClient::new(&e, &auditor).register_key(
        &AUDITOR_ID,
        &Grumpkin::generator(&e),
        &Address::generate(&e),
    );

    Harness {
        sac,
        verifier,
        auditor,
        admin: Address::generate(&e),
        freezer: Address::generate(&e),
        config_manager: Address::generate(&e),
        clawback: Address::generate(&e),
        e,
    }
}

impl Harness<'_> {
    fn deploy(
        &self,
        policy: Option<Address>,
        sac_passthrough: bool,
    ) -> CompliantTokenContractClient<'_> {
        let address = self.e.register(
            CompliantTokenContract,
            (
                self.admin.clone(),
                self.sac.address.clone(),
                self.verifier.clone(),
                self.auditor.clone(),
                ComplianceConfig { policy, sac_passthrough },
            ),
        );
        let token = CompliantTokenContractClient::new(&self.e, &address);
        token.grant_role(&self.freezer, &Symbol::new(&self.e, "freezer"), &self.admin);
        token.grant_role(
            &self.config_manager,
            &Symbol::new(&self.e, "config_manager"),
            &self.admin,
        );
        token.grant_role(&self.clawback, &Symbol::new(&self.e, "clawback"), &self.admin);
        token
    }

    fn allowlist(&self) -> MockAllowlistClient<'_> {
        MockAllowlistClient::new(&self.e, &self.e.register(MockAllowlist, ()))
    }
}

fn register(token: &CompliantTokenContractClient, account: &Address) {
    token.register(account, &AUDITOR_ID, &register_data(&token.env));
}

fn fund(h: &Harness, token: &CompliantTokenContractClient, account: &Address, amount: i128) {
    h.sac.mint(account, &amount);
    token.deposit(account, account, &amount);
    token.merge(account);
}

fn field(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[0u8; 32])
}

fn register_data(e: &Env) -> Bytes {
    RegisterData {
        payload: RegisterPayload { y: Grumpkin::generator(e), pvk: Grumpkin::generator(e) },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn withdraw_data(e: &Env) -> Bytes {
    WithdrawData {
        payload: WithdrawPayload {
            c_spend_new: Grumpkin::generator(e),
            b_tilde: field(e),
            r_e_point: Grumpkin::generator(e),
            sigma: field(e),
            b_tilde_aud_s: field(e),
            r_tilde_aud_s: field(e),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn transfer_data(e: &Env) -> Bytes {
    TransferData {
        payload: TransferPayload {
            c_spend_new: Grumpkin::generator(e),
            c_transfer: Grumpkin::generator(e),
            r_e_point: Grumpkin::generator(e),
            v_tilde: field(e),
            b_tilde: field(e),
            sigma: field(e),
            v_tilde_aud_r: field(e),
            r_tilde_aud_r: field(e),
            v_tilde_aud_s: field(e),
            b_tilde_aud_s: field(e),
            r_tilde_aud_s: field(e),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

/// Leaves `C_spend' = G` and escrows `C_a = G`.
fn set_spender_data(e: &Env) -> Bytes {
    SetSpenderData {
        payload: SetSpenderPayload {
            c_spend_new: Grumpkin::generator(e),
            c_a: Grumpkin::generator(e),
            escrowed_dvk: Grumpkin::generator(e),
            b_tilde: field(e),
            a_tilde: field(e),
            r_e_point: Grumpkin::generator(e),
            sigma: field(e),
            sigma_a: field(e),
            v_tilde_aud_s: field(e),
            b_tilde_aud_s: field(e),
            r_tilde_aud_s: field(e),
            r_a_tilde_aud_s: field(e),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn clawback_data(e: &Env) -> Bytes {
    ClawbackData { proof: Bytes::new(e) }.to_xdr(e)
}

fn unauthorized() -> Error {
    AccessControlError::Unauthorized.into()
}

// ################## FREEZE ##################

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn frozen_account_cannot_deposit() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    register(&token, &alice);
    h.sac.mint(&alice, &100);
    token.freeze(&alice, &h.freezer);

    token.deposit(&alice, &alice, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn frozen_account_cannot_transfer() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    register(&token, &alice);
    register(&token, &bob);
    fund(&h, &token, &alice, 100);
    token.freeze(&alice, &h.freezer);

    token.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn frozen_account_cannot_withdraw() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    register(&token, &alice);
    fund(&h, &token, &alice, 100);
    token.freeze(&alice, &h.freezer);

    token.withdraw(&alice, &alice, &40, &withdraw_data(&h.e));
}

#[test]
fn unfreeze_restores_a_frozen_account() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    register(&token, &alice);
    register(&token, &bob);
    h.sac.mint(&alice, &100);

    token.freeze(&alice, &h.freezer);
    assert!(token.is_frozen(&alice));
    token.unfreeze(&alice, &h.freezer);
    assert!(!token.is_frozen(&alice));

    token.deposit(&alice, &alice, &100);
    token.merge(&alice);
    token.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
    token.withdraw(&alice, &alice, &40, &withdraw_data(&h.e));
    assert_eq!(h.sac.balance(&alice), 40);
    assert_eq!(h.sac.balance(&token.address), 60);
}

// ################## POLICY ##################

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn unlisted_account_cannot_register() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);

    register(&token, &Address::generate(&h.e));
}

#[test]
fn listed_account_registers() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);
    allowlist.allow(&alice);

    register(&token, &alice);

    assert_eq!(token.confidential_balance(&alice).auditor_id, AUDITOR_ID);
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn disallowed_account_cannot_send() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    allowlist.allow(&alice);
    allowlist.allow(&bob);
    register(&token, &alice);
    register(&token, &bob);
    fund(&h, &token, &alice, 100);
    allowlist.disallow(&alice);

    token.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn disallowed_account_cannot_receive() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    allowlist.allow(&alice);
    allowlist.allow(&bob);
    register(&token, &alice);
    register(&token, &bob);
    fund(&h, &token, &alice, 100);
    allowlist.disallow(&bob);

    token.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
}

// ################## SPENDER GATING ##################

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn set_spender_to_denied_spender_fails() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    allowlist.allow(&alice);
    allowlist.allow(&spender);
    register(&token, &alice);
    register(&token, &spender);
    fund(&h, &token, &alice, 100);
    allowlist.disallow(&spender);

    token.set_spender(&alice, &spender, &1_000, &set_spender_data(&h.e));
}

#[test]
fn revoke_spender_succeeds_after_spender_is_denied() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    allowlist.allow(&alice);
    allowlist.allow(&spender);
    register(&token, &alice);
    register(&token, &spender);
    fund(&h, &token, &alice, 100);
    token.set_spender(&alice, &spender, &1_000, &set_spender_data(&h.e));
    allowlist.disallow(&spender);

    token.revoke_spender(&alice, &spender);

    assert!(!token.is_spender(&alice, &spender));
}

// ################## SAC PASSTHROUGH ##################

#[test]
#[should_panic(expected = "Error(Contract, #3603)")]
fn sac_deauthorized_account_is_blocked() {
    let h = setup();
    let token = h.deploy(None, true);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    register(&token, &alice);
    register(&token, &bob);
    fund(&h, &token, &alice, 100);
    h.sac.set_authorized(&alice, &false);

    token.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
}

// ################## CONFIG ROTATION ##################

#[test]
fn set_compliance_config_replaces_the_policy() {
    let h = setup();
    let old_policy = h.allowlist();
    let new_policy = h.allowlist();
    let token = h.deploy(Some(old_policy.address.clone()), false);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    old_policy.allow(&alice);
    new_policy.allow(&bob);

    let config =
        ComplianceConfig { policy: Some(new_policy.address.clone()), sac_passthrough: false };
    token.set_compliance_config(&config, &h.config_manager);

    assert_eq!(token.compliance_config(), Some(config));
    assert_eq!(
        token.try_register(&alice, &AUDITOR_ID, &register_data(&h.e)),
        Err(Ok(ComplianceError::NotAuthorizedByPolicy.into()))
    );
    register(&token, &bob);
}

#[test]
fn policy_none_disables_the_policy_gate() {
    let h = setup();
    let allowlist = h.allowlist();
    let token = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);

    token.set_compliance_config(
        &ComplianceConfig { policy: None, sac_passthrough: false },
        &h.config_manager,
    );

    register(&token, &alice);
}

// ################## ROLE SEPARATION ##################

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn freezer_cannot_clawback() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    register(&token, &alice);
    fund(&h, &token, &alice, 100);
    token.freeze(&alice, &h.freezer);

    token.clawback(&alice, &10, &None, &clawback_data(&h.e), &h.freezer);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn freezer_cannot_set_compliance_config() {
    let h = setup();
    let token = h.deploy(None, false);

    token.set_compliance_config(
        &ComplianceConfig { policy: None, sac_passthrough: true },
        &h.freezer,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn clawback_holder_cannot_freeze() {
    let h = setup();
    let token = h.deploy(None, false);

    token.freeze(&Address::generate(&h.e), &h.clawback);
}

#[test]
fn caller_without_a_role_is_rejected() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    let config = ComplianceConfig { policy: None, sac_passthrough: false };

    for caller in [h.admin.clone(), Address::generate(&h.e)] {
        assert_eq!(token.try_freeze(&alice, &caller), Err(Ok(unauthorized())));
        assert_eq!(token.try_unfreeze(&alice, &caller), Err(Ok(unauthorized())));
        assert_eq!(token.try_set_compliance_config(&config, &caller), Err(Ok(unauthorized())));
        assert_eq!(
            token.try_clawback(&alice, &10, &None, &clawback_data(&h.e), &caller),
            Err(Ok(unauthorized()))
        );
        assert_eq!(
            token.try_force_revoke_spender(&alice, &spender, &caller),
            Err(Ok(unauthorized()))
        );
    }
}

// ################## CLAWBACK ##################

#[test]
#[should_panic(expected = "Error(Contract, #3604)")]
fn clawback_on_unfrozen_account_fails() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    register(&token, &alice);
    fund(&h, &token, &alice, 100);

    token.clawback(&alice, &10, &None, &clawback_data(&h.e), &h.clawback);
}

#[test]
#[should_panic(expected = "Error(Contract, #3606)")]
fn clawback_to_the_token_address_fails() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    register(&token, &alice);
    fund(&h, &token, &alice, 100);
    token.freeze(&alice, &h.freezer);

    token.clawback(&alice, &10, &Some(token.address.clone()), &clawback_data(&h.e), &h.clawback);
}

#[test]
fn force_revoke_spender_folds_the_allowance_into_spendable() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    register(&token, &alice);
    register(&token, &spender);
    fund(&h, &token, &alice, 100);
    token.set_spender(&alice, &spender, &1_000, &set_spender_data(&h.e));
    token.freeze(&alice, &h.freezer);

    token.force_revoke_spender(&alice, &spender, &h.clawback);

    let g = Grumpkin::generator(&h.e);
    assert_eq!(
        token.confidential_balance(&alice).spendable_commitment,
        Grumpkin::add(&h.e, &g, &g)
    );
    assert!(token.try_get_spender_delegation(&alice, &spender).is_err());
}

#[test]
fn clawback_nonce_advances_per_seizure() {
    let h = setup();
    let token = h.deploy(None, false);
    let alice = Address::generate(&h.e);
    let destination = Address::generate(&h.e);
    register(&token, &alice);
    fund(&h, &token, &alice, 100);
    token.freeze(&alice, &h.freezer);
    assert_eq!(token.clawback_nonce(&alice), 0);

    token.clawback(&alice, &30, &None, &clawback_data(&h.e), &h.clawback);
    assert_eq!(token.clawback_nonce(&alice), 1);

    token.clawback(&alice, &20, &Some(destination.clone()), &clawback_data(&h.e), &h.clawback);
    assert_eq!(token.clawback_nonce(&alice), 2);

    let g = Grumpkin::generator(&h.e);
    assert_eq!(
        token.confidential_balance(&alice).spendable_commitment,
        Grumpkin::mul(&h.e, &g, 50)
    );
    assert_eq!(h.sac.balance(&destination), 20);
    assert_eq!(h.sac.balance(&token.address), 80);
}

// ################## SHARED REGISTRY ##################

#[test]
fn one_allowlist_gates_two_tokens() {
    let h = setup();
    let allowlist = h.allowlist();
    let token_a = h.deploy(Some(allowlist.address.clone()), false);
    let token_b = h.deploy(Some(allowlist.address.clone()), false);
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    allowlist.allow(&alice);

    register(&token_a, &alice);
    register(&token_b, &alice);
    for token in [&token_a, &token_b] {
        assert_eq!(
            token.try_register(&bob, &AUDITOR_ID, &register_data(&h.e)),
            Err(Ok(ComplianceError::NotAuthorizedByPolicy.into()))
        );
    }

    allowlist.allow(&bob);
    register(&token_a, &bob);
    register(&token_b, &bob);
}
