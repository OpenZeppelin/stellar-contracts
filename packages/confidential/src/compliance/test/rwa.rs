extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events, MockAuth, MockAuthInvoke},
    token::StellarAssetClient,
    xdr::{AccountFlags, ToXdr},
    Address, Bytes, BytesN, Env, Error, Event, IntoVal, Val, Vec,
};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::{
    compliance::{
        storage::{
            compliance_config, deposit_approval_args, freeze, rwa_config, set_compliance_config,
            set_rwa_config, spender_transfer_approval_args, transfer_approval_args, unfreeze,
            withdraw_approval_args,
        },
        ComplianceConfig, ComplianceError, ComplianceStorageKey, ConfidentialCompliance,
        ConfidentialComplianceClient, ConfidentialRwa, ConfidentialRwaClient, Policy, RwaConfig,
        RwaConfigChanged, RwaHooks,
    },
    storage::{
        get_account, get_auditor, set_address_as_field_element, set_auditor, set_underlying_asset,
        set_verifier,
    },
    verifier::CircuitType,
    ConfidentialAccount, ConfidentialToken, ConfidentialTokenClient, ConfidentialTokenStorageKey,
    Hooks, RegisterData, RegisterPayload, SetSpenderData, SetSpenderPayload, SpenderDelegation,
    SpenderTransferData, SpenderTransferPayload, TransferData, TransferPayload, WithdrawData,
    WithdrawPayload,
};

// ################## MOCK CONTRACTS ##################

#[contract]
struct RwaHost;

#[contractimpl]
impl RwaHost {
    pub fn __constructor(e: &Env, token: Address, verifier: Address, auditor: Address) {
        set_underlying_asset(e, &token);
        set_verifier(e, &verifier);
        set_auditor(e, &auditor);
        set_address_as_field_element(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialToken for RwaHost {
    type Hooks = RwaHooks;
}

#[contractimpl(contracttrait)]
impl ConfidentialCompliance for RwaHost {
    fn freeze(e: &Env, account: Address, admin: Address) {
        admin.require_auth();
        freeze(e, &account);
    }

    fn unfreeze(e: &Env, account: Address, admin: Address) {
        admin.require_auth();
        unfreeze(e, &account);
    }

    fn set_compliance_config(e: &Env, config: ComplianceConfig, admin: Address) {
        admin.require_auth();
        set_compliance_config(e, &config);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialRwa for RwaHost {
    fn set_rwa_config(e: &Env, auditor_id: u32, approver: Option<Address>, admin: Address) {
        admin.require_auth();
        set_rwa_config(e, auditor_id, &approver);
    }
}

/// Authorizes every address except the one set through `block`.
#[contract]
struct ListPolicy;

#[contractimpl]
impl ListPolicy {
    pub fn block(e: &Env, account: Address) {
        e.storage().instance().set(&symbol_short!("blocked"), &account);
    }
}

#[contractimpl]
impl Policy for ListPolicy {
    fn is_authorized(e: Env, account: Address, _token: Address) -> bool {
        e.storage().instance().get::<_, Address>(&symbol_short!("blocked")) != Some(account)
    }
}

/// A policy whose call always fails.
#[contract]
struct PanicPolicy;

#[contractimpl]
impl Policy for PanicPolicy {
    fn is_authorized(_e: Env, _account: Address, _token: Address) -> bool {
        panic!("policy unavailable")
    }
}

/// Accepts every proof.
#[contract]
struct AcceptingVerifier;

#[contractimpl(contracttrait)]
impl crate::verifier::ConfidentialVerifier for AcceptingVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(_e: &Env, _ct: CircuitType, _pi: Bytes, _proof: Bytes) -> bool {
        true
    }
}

#[contract]
struct MockAuditor;

#[contractimpl(contracttrait)]
impl crate::auditor::ConfidentialAuditor for MockAuditor {
    fn register_key(e: &Env, auditor_id: u32, point: BytesN<64>, _operator: Address) {
        crate::auditor::storage::register_key(e, auditor_id, &point);
    }

    fn rotate_key(_e: &Env, _auditor_id: u32, _new_point: BytesN<64>, _operator: Address) {}
}

// ################## SETUP ##################

/// The designated auditor id of every configured harness.
const AUDITOR: u32 = 0;
/// A second registered auditor id, standing for a legacy binding.
const LEGACY_AUDITOR: u32 = 1;

struct Harness<'a> {
    e: Env,
    host: Address,
    verifier: Address,
    sac_addr: Address,
    sac: StellarAssetClient<'a>,
    policy: Address,
    admin: Address,
}

fn setup<'a>() -> Harness<'a> {
    setup_with(|e| e.register(AcceptingVerifier, ()))
}

fn setup_with<'a>(register_verifier: impl FnOnce(&Env) -> Address) -> Harness<'a> {
    let e = Env::default();
    e.mock_all_auths();

    let issuer = Address::generate(&e);
    let sac = e.register_stellar_asset_contract_v2(issuer);
    let sac_addr = sac.address();
    // SAC v2 honours `set_authorized` only when the issuer is revocable.
    sac.issuer().set_flag(AccountFlags::RevocableFlag);
    let sac_client = StellarAssetClient::new(&e, &sac_addr);

    let verifier = register_verifier(&e);
    let auditor = e.register(MockAuditor, ());
    let admin = Address::generate(&e);
    let auditor_client = crate::auditor::ConfidentialAuditorClient::new(&e, &auditor);
    auditor_client.register_key(&AUDITOR, &Grumpkin::generator(&e), &admin);
    // A distinct key, so tests can tell which auditor's key a proof binds.
    auditor_client.register_key(
        &LEGACY_AUDITOR,
        &Grumpkin::mul(&e, &Grumpkin::generator(&e), 2),
        &admin,
    );

    let policy = e.register(ListPolicy, ());
    let host = e.register(RwaHost, (sac_addr.clone(), verifier.clone(), auditor));

    Harness { e, host, verifier, sac_addr, sac: sac_client, policy, admin }
}

/// A harness with both configurations set: a policy gate, the designated
/// auditor, and `approver`.
fn configured<'a>(approver: Option<Address>) -> Harness<'a> {
    let h = setup();
    configure(&h, approver);
    h
}

fn configure(h: &Harness, approver: Option<Address>) {
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &policy_config(h));
        set_rwa_config(&h.e, AUDITOR, &approver);
    });
}

fn policy_config(h: &Harness) -> ComplianceConfig {
    ComplianceConfig { policy: Some(h.policy.clone()), sac_passthrough: false }
}

fn point(e: &Env) -> BytesN<64> {
    Grumpkin::generator(e)
}

fn field(e: &Env, byte: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[31] = byte;
    BytesN::from_array(e, &b)
}

fn g_times(e: &Env, n: u128) -> BytesN<64> {
    Grumpkin::mul(e, &Grumpkin::generator(e), n)
}

/// Writes an account record directly, bypassing registration.
fn store_account(
    h: &Harness,
    account: &Address,
    auditor_id: u32,
    spendable: u128,
    receiving: u128,
) {
    h.e.as_contract(&h.host, || {
        let acc = ConfidentialAccount {
            spending_public_key: point(&h.e),
            viewing_public_key: point(&h.e),
            spendable_commitment: g_times(&h.e, spendable),
            receiving_commitment: g_times(&h.e, receiving),
            auditor_id,
        };
        h.e.storage()
            .persistent()
            .set(&ConfidentialTokenStorageKey::Account(account.clone()), &acc);
    });
}

fn account(h: &Harness, auditor_id: u32) -> Address {
    let a = Address::generate(&h.e);
    store_account(h, &a, auditor_id, 0, 0);
    a
}

fn store_delegation(h: &Harness, owner: &Address, spender: &Address, allowance: u128) {
    h.e.as_contract(&h.host, || {
        let d = SpenderDelegation {
            allowance_commitment: g_times(&h.e, allowance),
            a_tilde: field(&h.e, 1),
            escrowed_dvk: point(&h.e),
            allowance_salt: field(&h.e, 2),
            live_until_ledger: u32::MAX,
        };
        h.e.storage()
            .persistent()
            .set(&ConfidentialTokenStorageKey::Delegation(owner.clone(), spender.clone()), &d);
    });
}

fn register_payload(e: &Env) -> RegisterPayload {
    RegisterPayload { y: point(e), pvk: point(e) }
}

fn withdraw_payload(e: &Env) -> WithdrawPayload {
    WithdrawPayload {
        c_spend_new: point(e),
        b_tilde: field(e, 1),
        r_e_point: point(e),
        sigma: field(e, 2),
        b_tilde_aud_s: field(e, 3),
        r_tilde_aud_s: field(e, 4),
    }
}

fn transfer_payload(e: &Env) -> TransferPayload {
    TransferPayload {
        c_spend_new: point(e),
        c_transfer: point(e),
        r_e_point: point(e),
        v_tilde: field(e, 1),
        b_tilde: field(e, 2),
        sigma: field(e, 3),
        v_tilde_aud_r: field(e, 4),
        r_tilde_aud_r: field(e, 5),
        v_tilde_aud_s: field(e, 6),
        b_tilde_aud_s: field(e, 7),
        r_tilde_aud_s: field(e, 8),
    }
}

fn spender_transfer_payload(e: &Env) -> SpenderTransferPayload {
    SpenderTransferPayload {
        c_a_new: point(e),
        c_transfer: point(e),
        r_e_point: point(e),
        v_tilde: field(e, 1),
        a_tilde_new: field(e, 2),
        sigma_a_new: field(e, 3),
        v_tilde_aud_r: field(e, 4),
        r_tilde_aud_r: field(e, 5),
        v_tilde_aud_s: field(e, 6),
        a_tilde_aud_s: field(e, 7),
        r_tilde_aud_s: field(e, 8),
    }
}

fn set_spender_payload(e: &Env) -> SetSpenderPayload {
    SetSpenderPayload {
        c_spend_new: point(e),
        c_a: point(e),
        escrowed_dvk: point(e),
        b_tilde: field(e, 1),
        a_tilde: field(e, 2),
        r_e_point: point(e),
        sigma: field(e, 3),
        sigma_a: field(e, 4),
        v_tilde_aud_s: field(e, 5),
        b_tilde_aud_s: field(e, 6),
        r_tilde_aud_s: field(e, 7),
        r_a_tilde_aud_s: field(e, 8),
    }
}

fn transfer_data(e: &Env) -> Bytes {
    TransferData { payload: transfer_payload(e), proof: Bytes::new(e) }.to_xdr(e)
}

fn withdraw_data(e: &Env) -> Bytes {
    WithdrawData { payload: withdraw_payload(e), proof: Bytes::new(e) }.to_xdr(e)
}

fn spender_transfer_data(e: &Env) -> Bytes {
    SpenderTransferData { payload: spender_transfer_payload(e), proof: Bytes::new(e) }.to_xdr(e)
}

fn set_spender_data(e: &Env) -> Bytes {
    SetSpenderData { payload: set_spender_payload(e), proof: Bytes::new(e) }.to_xdr(e)
}

fn freeze_account(h: &Harness, account: &Address) {
    h.e.as_contract(&h.host, || freeze(&h.e, account));
}

/// Runs every `RwaHooks` callback once against `a` and `b`.
fn run_hook(h: &Harness, which: u32, a: &Address, b: &Address) {
    let e = &h.e;
    e.as_contract(&h.host, || match which {
        0 => RwaHooks::on_register(e, a, AUDITOR, &register_payload(e)),
        1 => RwaHooks::on_deposit(e, a, b, 1),
        2 => RwaHooks::on_merge(e, a),
        3 => RwaHooks::on_withdraw(e, a, b, 1, &withdraw_payload(e)),
        4 => RwaHooks::on_transfer(e, a, b, &transfer_payload(e)),
        5 => RwaHooks::on_spender_transfer(e, b, a, b, &spender_transfer_payload(e)),
        6 => RwaHooks::on_set_spender(e, a, b, 100, &set_spender_payload(e)),
        _ => RwaHooks::on_revoke_spender(e, a, b),
    });
}

const HOOK_COUNT: u32 = 8;

/// Runs hook `which` and returns the panic message, or `None` when it passes.
fn hook_panic(h: &Harness, which: u32, a: &Address, b: &Address) -> Option<std::string::String> {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_hook(h, which, a, b)));
    res.err().map(|payload| {
        payload
            .downcast_ref::<std::string::String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| std::string::String::from(*s)))
            .unwrap_or_default()
    })
}

/// Asserts that every hook reverts with the contract error `code`.
fn assert_every_hook_reverts_with(h: &Harness, a: &Address, b: &Address, code: u32) {
    let expected = std::format!("Error(Contract, #{code})");
    for which in 0..HOOK_COUNT {
        let msg = hook_panic(h, which, a, b)
            .unwrap_or_else(|| panic!("hook {which} passed, expected {expected}"));
        assert!(msg.contains(&expected), "hook {which}: expected {expected}, got {msg}");
    }
}

// ################## CONFIGURATION ##################

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn set_rwa_config_requires_compliance_config() {
    let h = setup();
    h.e.as_contract(&h.host, || set_rwa_config(&h.e, AUDITOR, &None));
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
fn set_rwa_config_requires_eligibility_gate() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: false });
        set_rwa_config(&h.e, AUDITOR, &None);
    });
}

#[test]
fn set_rwa_config_accepts_sac_passthrough_as_gate() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: true });
        set_rwa_config(&h.e, AUDITOR, &None);
        assert_eq!(rwa_config(&h.e), Some(RwaConfig { auditor_id: AUDITOR, approver: None }));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn set_rwa_config_rejects_unregistered_auditor() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &policy_config(&h));
        set_rwa_config(&h.e, 7, &None);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn set_rwa_config_rejects_auditor_change() {
    let h = configured(None);
    h.e.as_contract(&h.host, || set_rwa_config(&h.e, LEGACY_AUDITOR, &None));
}

#[test]
#[should_panic(expected = "Error(Contract, #3610)")]
fn set_rwa_config_rejects_token_as_approver() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &policy_config(&h));
        set_rwa_config(&h.e, AUDITOR, &Some(h.host.clone()));
    });
}

#[test]
fn set_rwa_config_stores_and_emits() {
    let h = setup();
    let approver = Address::generate(&h.e);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    ConfidentialComplianceClient::new(&h.e, &h.host)
        .set_compliance_config(&policy_config(&h), &h.admin);
    assert_eq!(client.rwa_config(), None);

    client.set_rwa_config(&AUDITOR, &Some(approver.clone()), &h.admin);

    let events = h.e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &RwaConfigChanged { auditor_id: AUDITOR, approver: Some(approver.clone()) }
            .to_xdr(&h.e, &h.host),
    );
    assert_eq!(
        client.rwa_config(),
        Some(RwaConfig { auditor_id: AUDITOR, approver: Some(approver) })
    );
}

#[test]
fn approver_can_be_changed_and_disabled() {
    let h = configured(None);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    let a = Address::generate(&h.e);

    client.set_rwa_config(&AUDITOR, &Some(a.clone()), &h.admin);
    assert_eq!(client.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR, approver: Some(a) }));
    client.set_rwa_config(&AUDITOR, &None, &h.admin);
    assert_eq!(client.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR, approver: None }));
}

#[test]
fn set_compliance_config_is_unchanged_without_rwa_config() {
    let h = setup();
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);
    let open = ComplianceConfig { policy: None, sac_passthrough: false };
    client.set_compliance_config(&open, &h.admin);

    assert_eq!(h.e.events().all().events().len(), 1);
    h.e.as_contract(&h.host, || {
        assert_eq!(compliance_config(&h.e), Some(open.clone()));
    });
}

// ################## FAIL CLOSED ##################

#[test]
fn every_hook_reverts_without_rwa_config() {
    let h = setup();
    h.e.as_contract(&h.host, || set_compliance_config(&h.e, &policy_config(&h)));
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    assert_every_hook_reverts_with(&h, &a, &b, 3607);
}

#[test]
fn every_hook_reverts_without_compliance_config() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || h.e.storage().instance().remove(&ComplianceStorageKey::Config));
    assert_every_hook_reverts_with(&h, &a, &b, 3600);
}

#[test]
fn every_hook_reverts_without_eligibility_gate() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    let open = ComplianceConfig { policy: None, sac_passthrough: false };
    ConfidentialComplianceClient::new(&h.e, &h.host).set_compliance_config(&open, &h.admin);
    assert_every_hook_reverts_with(&h, &a, &b, 3612);
}

#[test]
fn every_hook_passes_when_configured_without_approver() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    for which in 0..HOOK_COUNT {
        run_hook(&h, which, &a, &b);
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #3607)")]
fn on_transfer_without_rwa_config_panics_rwa_not_configured() {
    let h = setup();
    h.e.as_contract(&h.host, || set_compliance_config(&h.e, &policy_config(&h)));
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    run_hook(&h, 4, &a, &b);
}

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn on_transfer_without_compliance_config_panics_not_configured() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || h.e.storage().instance().remove(&ComplianceStorageKey::Config));
    run_hook(&h, 4, &a, &b);
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
fn on_transfer_without_eligibility_gate_panics() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(
            &ComplianceStorageKey::Config,
            &ComplianceConfig { policy: None, sac_passthrough: false },
        );
    });
    run_hook(&h, 4, &a, &b);
}

// ################## DESIGNATED AUDITOR ##################

#[test]
#[should_panic(expected = "Error(Contract, #3608)")]
fn on_register_rejects_other_auditor() {
    let h = configured(None);
    let a = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_register(&h.e, &a, LEGACY_AUDITOR, &register_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3608)")]
fn register_entry_point_rejects_other_auditor() {
    let h = configured(None);
    let a = Address::generate(&h.e);
    let data = RegisterData { payload: register_payload(&h.e), proof: Bytes::new(&h.e) };
    ConfidentialTokenClient::new(&h.e, &h.host).register(&a, &LEGACY_AUDITOR, &data.to_xdr(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_register_rejects_policy_denied_account() {
    let h = configured(None);
    let a = Address::generate(&h.e);
    ListPolicyClient::new(&h.e, &h.policy).block(&a);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_register(&h.e, &a, AUDITOR, &register_payload(&h.e));
    });
}

// ################## ELIGIBILITY GATES ##################

#[test]
fn on_deposit_accepts_unregistered_depositor() {
    let h = configured(None);
    let from = Address::generate(&h.e);
    let to = account(&h, AUDITOR);
    run_hook(&h, 1, &from, &to);
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_set_spender_rejects_frozen_spender() {
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    freeze_account(&h, &spender);
    run_hook(&h, 6, &owner, &spender);
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_spender_transfer_rejects_frozen_spender() {
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let to = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    freeze_account(&h, &spender);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(&h.e, &spender, &owner, &to, &spender_transfer_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_spender_transfer_rejects_policy_denied_spender() {
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let to = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    ListPolicyClient::new(&h.e, &h.policy).block(&spender);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(&h.e, &spender, &owner, &to, &spender_transfer_payload(&h.e));
    });
}

#[test]
fn on_revoke_spender_ignores_frozen_and_denied_spender() {
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    freeze_account(&h, &spender);
    ListPolicyClient::new(&h.e, &h.policy).block(&spender);
    run_hook(&h, 7, &owner, &spender);
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_merge_rejects_frozen_account() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    freeze_account(&h, &a);
    run_hook(&h, 2, &a, &a);
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_withdraw_rejects_policy_denied_recipient() {
    let h = configured(None);
    let from = account(&h, AUDITOR);
    let to = Address::generate(&h.e);
    ListPolicyClient::new(&h.e, &h.policy).block(&to);
    run_hook(&h, 3, &from, &to);
}

/// Which party of a hook a gate test makes ineligible.
#[derive(Clone, Copy)]
enum Party {
    A,
    B,
}

/// Runs hook `which` with one party frozen and asserts `AccountFrozen`.
fn assert_frozen_party_rejected(which: u32, party: Party) {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    freeze_account(
        &h,
        match party {
            Party::A => &a,
            Party::B => &b,
        },
    );
    let msg = hook_panic(&h, which, &a, &b).expect("hook passed with a frozen party");
    assert!(msg.contains("Error(Contract, #3601)"), "hook {which}: {msg}");
}

/// Runs hook `which` with one party denied by the policy and asserts
/// `NotAuthorizedByPolicy`.
fn assert_denied_party_rejected(which: u32, party: Party) {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    ListPolicyClient::new(&h.e, &h.policy).block(match party {
        Party::A => &a,
        Party::B => &b,
    });
    let msg = hook_panic(&h, which, &a, &b).expect("hook passed with a policy-denied party");
    assert!(msg.contains("Error(Contract, #3602)"), "hook {which}: {msg}");
}

#[test]
fn every_gated_party_is_rejected_when_frozen() {
    // (hook, party): deposit from/to, merge account, withdraw from/to,
    // transfer from/to, spender transfer owner/recipient-and-spender,
    // set_spender account/spender, revoke account.
    for (which, party) in [
        (1, Party::A),
        (1, Party::B),
        (2, Party::A),
        (3, Party::A),
        (3, Party::B),
        (4, Party::A),
        (4, Party::B),
        (5, Party::A),
        (5, Party::B),
        (6, Party::A),
        (6, Party::B),
        (7, Party::A),
    ] {
        assert_frozen_party_rejected(which, party);
    }
}

#[test]
fn every_gated_party_is_rejected_when_policy_denied() {
    for (which, party) in [
        (0, Party::A),
        (1, Party::A),
        (1, Party::B),
        (2, Party::A),
        (3, Party::A),
        (3, Party::B),
        (4, Party::A),
        (4, Party::B),
        (5, Party::A),
        (5, Party::B),
        (6, Party::A),
        (6, Party::B),
        (7, Party::A),
    ] {
        assert_denied_party_rejected(which, party);
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_spender_transfer_rejects_frozen_recipient() {
    // In `run_hook` the recipient doubles as the spender; here they differ.
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let to = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    freeze_account(&h, &to);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(&h.e, &spender, &owner, &to, &spender_transfer_payload(&h.e));
    });
}

#[test]
fn hooks_reject_sac_unauthorized_parties_under_passthrough() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: true });
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    let a = account(&h, AUDITOR);
    let b = account(&h, AUDITOR);
    h.sac.set_authorized(&b, &false);
    for which in [0, 2, 7] {
        // `b` is the gated account of register, merge and revoke here.
        let msg = hook_panic(&h, which, &b, &a).expect("hook passed for a SAC-denied account");
        assert!(msg.contains("Error(Contract, #3603)"), "hook {which}: {msg}");
    }
    for which in [1, 3, 4, 5, 6] {
        let msg = hook_panic(&h, which, &a, &b).expect("hook passed for a SAC-denied party");
        assert!(msg.contains("Error(Contract, #3603)"), "hook {which}: {msg}");
    }
}

// ################## APPROVAL GATE ##################

/// Registered, funded sender and a registered recipient on a deployment whose
/// approver is `approver`.
struct Flow<'a> {
    h: Harness<'a>,
    approver: Address,
    alice: Address,
    bob: Address,
}

fn approval_flow<'a>() -> Flow<'a> {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    let alice = account(&h, AUDITOR);
    let bob = account(&h, AUDITOR);
    store_account(&h, &alice, AUDITOR, 1_000, 0);
    h.sac.mint(&alice, &1_000);
    h.sac.mint(&h.host, &1_000);
    Flow { h, approver, alice, bob }
}

fn invoke<'a>(
    contract: &'a Address,
    fn_name: &'a str,
    args: Vec<Val>,
    sub_invokes: &'a [MockAuthInvoke<'a>],
) -> MockAuthInvoke<'a> {
    MockAuthInvoke { contract, fn_name, args, sub_invokes }
}

/// Runs `deposit(alice, bob, amount)` with exactly the sender's authorization
/// and, when given, an approver authorization over `approver_args`.
fn deposit_with(f: &Flow, amount: i128, approver_args: Option<Vec<Val>>) {
    let e = &f.h.e;
    let sac_transfer = [invoke(
        &f.h.sac_addr,
        "transfer",
        (f.alice.clone(), f.h.host.clone(), amount).into_val(e),
        &[],
    )];
    let sender = invoke(
        &f.h.host,
        "deposit",
        (f.alice.clone(), f.bob.clone(), amount).into_val(e),
        &sac_transfer,
    );
    let token = ConfidentialTokenClient::new(e, &f.h.host);
    match approver_args {
        None => {
            token
                .mock_auths(&[MockAuth { address: &f.alice, invoke: &sender }])
                .deposit(&f.alice, &f.bob, &amount);
        }
        Some(args) => {
            let approval = invoke(&f.h.host, "deposit", args, &[]);
            token
                .mock_auths(&[
                    MockAuth { address: &f.alice, invoke: &sender },
                    MockAuth { address: &f.approver, invoke: &approval },
                ])
                .deposit(&f.alice, &f.bob, &amount);
        }
    }
}

fn current_deposit_args(f: &Flow, amount: i128) -> Vec<Val> {
    f.h.e.as_contract(&f.h.host, || deposit_approval_args(&f.h.e, &f.alice, &f.bob, amount))
}

#[test]
fn deposit_with_exact_approval_succeeds() {
    let f = approval_flow();
    deposit_with(&f, 100, Some(current_deposit_args(&f, 100)));

    let auths = f.h.e.auths();
    assert_eq!(auths.len(), 2);
    assert!(auths.iter().any(|(addr, _)| addr == &f.approver));
    f.h.e.as_contract(&f.h.host, || {
        assert_eq!(get_account(&f.h.e, &f.bob).receiving_commitment, g_times(&f.h.e, 100));
    });
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_without_approval_fails() {
    let f = approval_flow();
    deposit_with(&f, 100, None);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_for_other_amount_fails() {
    let f = approval_flow();
    deposit_with(&f, 100, Some(current_deposit_args(&f, 99)));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_for_other_recipient_fails() {
    let f = approval_flow();
    let carol = account(&f.h, AUDITOR);
    let args =
        f.h.e.as_contract(&f.h.host, || deposit_approval_args(&f.h.e, &f.alice, &carol, 100));
    deposit_with(&f, 100, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn approval_does_not_survive_approver_replacement() {
    let f = approval_flow();
    let stale = current_deposit_args(&f, 100);
    let replacement = Some(Address::generate(&f.h.e));
    ConfidentialRwaClient::new(&f.h.e, &f.h.host).set_rwa_config(
        &AUDITOR,
        &replacement,
        &f.h.admin,
    );
    deposit_with(&f, 100, Some(stale));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_under_another_function_name_fails() {
    // The exact `withdraw` tuple, authorized for `deposit`: only the
    // function name differs from the passing case
    // (`withdraw_approval_tuple_is_from_to_amount_payload`).
    let f = approval_flow();
    let e = &f.h.e;
    let data = withdraw_data(e);
    let sender = invoke(
        &f.h.host,
        "withdraw",
        (f.alice.clone(), f.alice.clone(), 100i128, data.clone()).into_val(e),
        &[],
    );
    let args: Vec<Val> =
        (f.alice.clone(), f.alice.clone(), 100i128, withdraw_payload(e)).into_val(e);
    let approval = invoke(&f.h.host, "deposit", args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &f.alice, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .withdraw(&f.alice, &f.alice, &100, &data);
}

#[test]
fn withdraw_with_exact_approval_succeeds() {
    let f = approval_flow();
    let e = &f.h.e;
    let data = withdraw_data(e);
    let sender = invoke(
        &f.h.host,
        "withdraw",
        (f.alice.clone(), f.alice.clone(), 100i128, data.clone()).into_val(e),
        &[],
    );
    let args = e.as_contract(&f.h.host, || {
        withdraw_approval_args(e, &f.alice, &f.alice, 100, &withdraw_payload(e))
    });
    let approval = invoke(&f.h.host, "withdraw", args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &f.alice, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .withdraw(&f.alice, &f.alice, &100, &data);

    assert_eq!(StellarAssetClient::new(e, &f.h.sac_addr).balance(&f.alice), 1_100);
}

fn transfer_with(f: &Flow, approver_args: Option<Vec<Val>>) {
    let e = &f.h.e;
    let data = transfer_data(e);
    let sender = invoke(
        &f.h.host,
        "confidential_transfer",
        (f.alice.clone(), f.bob.clone(), data.clone()).into_val(e),
        &[],
    );
    let token = ConfidentialTokenClient::new(e, &f.h.host);
    match approver_args {
        None => {
            token
                .mock_auths(&[MockAuth { address: &f.alice, invoke: &sender }])
                .confidential_transfer(&f.alice, &f.bob, &data);
        }
        Some(args) => {
            let approval = invoke(&f.h.host, "confidential_transfer", args, &[]);
            token
                .mock_auths(&[
                    MockAuth { address: &f.alice, invoke: &sender },
                    MockAuth { address: &f.approver, invoke: &approval },
                ])
                .confidential_transfer(&f.alice, &f.bob, &data);
        }
    }
}

#[test]
fn transfer_with_exact_approval_succeeds() {
    let f = approval_flow();
    let e = &f.h.e;
    let args = e.as_contract(&f.h.host, || {
        transfer_approval_args(e, &f.alice, &f.bob, &transfer_payload(e))
    });
    transfer_with(&f, Some(args));

    e.as_contract(&f.h.host, || {
        assert_eq!(get_account(e, &f.bob).receiving_commitment, point(e));
    });
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_without_approval_fails() {
    let f = approval_flow();
    transfer_with(&f, None);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_for_other_payload_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let mut other = transfer_payload(e);
    other.c_transfer = g_times(e, 2);
    let args = e.as_contract(&f.h.host, || transfer_approval_args(e, &f.alice, &f.bob, &other));
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_for_other_sender_side_payload_fails() {
    // The approval binds the whole payload, not only `c_transfer`.
    let f = approval_flow();
    let e = &f.h.e;
    let mut other = transfer_payload(e);
    other.c_spend_new = g_times(e, 2);
    let args = e.as_contract(&f.h.host, || transfer_approval_args(e, &f.alice, &f.bob, &other));
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_under_another_function_name_fails() {
    // The exact `confidential_transfer_from` tuple, authorized for
    // `confidential_transfer`: only the function name differs from the
    // passing case
    // (`spender_transfer_approval_tuple_is_spender_from_to_payload`).
    let f = approval_flow();
    let e = &f.h.e;
    let spender = account(&f.h, AUDITOR);
    store_delegation(&f.h, &f.alice, &spender, 500);
    let data = spender_transfer_data(e);
    let sender = invoke(
        &f.h.host,
        "confidential_transfer_from",
        (spender.clone(), f.alice.clone(), f.bob.clone(), data.clone()).into_val(e),
        &[],
    );
    let args: Vec<Val> =
        (spender.clone(), f.alice.clone(), f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    let approval = invoke(&f.h.host, "confidential_transfer", args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &spender, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .confidential_transfer_from(&spender, &f.alice, &f.bob, &data);
}

#[test]
fn spender_transfer_with_exact_approval_succeeds() {
    let f = approval_flow();
    let e = &f.h.e;
    let spender = account(&f.h, AUDITOR);
    store_delegation(&f.h, &f.alice, &spender, 500);
    let data = spender_transfer_data(e);
    let sender = invoke(
        &f.h.host,
        "confidential_transfer_from",
        (spender.clone(), f.alice.clone(), f.bob.clone(), data.clone()).into_val(e),
        &[],
    );
    let args = e.as_contract(&f.h.host, || {
        spender_transfer_approval_args(e, &spender, &f.alice, &f.bob, &spender_transfer_payload(e))
    });
    let approval = invoke(&f.h.host, "confidential_transfer_from", args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &spender, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .confidential_transfer_from(&spender, &f.alice, &f.bob, &data);

    e.as_contract(&f.h.host, || {
        assert_eq!(get_account(e, &f.bob).receiving_commitment, point(e));
    });
}

/// Runs `withdraw(alice, alice, 100, data)` with the sender's authorization
/// and an approver authorization over `approver_args`.
fn withdraw_with(f: &Flow, approver_args: Vec<Val>) {
    let e = &f.h.e;
    let data = withdraw_data(e);
    let sender = invoke(
        &f.h.host,
        "withdraw",
        (f.alice.clone(), f.alice.clone(), 100i128, data.clone()).into_val(e),
        &[],
    );
    let approval = invoke(&f.h.host, "withdraw", approver_args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &f.alice, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .withdraw(&f.alice, &f.alice, &100, &data);
}

/// Runs `confidential_transfer_from(spender, alice, bob, data)` with the
/// spender's authorization and an approver authorization over
/// `approver_args`.
fn spender_transfer_with(f: &Flow, spender: &Address, approver_args: Vec<Val>) {
    let e = &f.h.e;
    let data = spender_transfer_data(e);
    let sender = invoke(
        &f.h.host,
        "confidential_transfer_from",
        (spender.clone(), f.alice.clone(), f.bob.clone(), data.clone()).into_val(e),
        &[],
    );
    let approval = invoke(&f.h.host, "confidential_transfer_from", approver_args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: spender, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .confidential_transfer_from(spender, &f.alice, &f.bob, &data);
}

fn delegated_flow<'a>() -> (Flow<'a>, Address) {
    let f = approval_flow();
    let spender = account(&f.h, AUDITOR);
    store_delegation(&f.h, &f.alice, &spender, 500);
    (f, spender)
}

// The four tests below build the approval tuple by hand, so the wire shape
// of each tuple is pinned independently of the library's own helpers.

#[test]
fn deposit_approval_tuple_is_from_to_amount() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (f.alice.clone(), f.bob.clone(), 100i128).into_val(e);
    deposit_with(&f, 100, Some(args));
}

#[test]
fn withdraw_approval_tuple_is_from_to_amount_payload() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (f.alice.clone(), f.alice.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
fn transfer_approval_tuple_is_from_to_payload() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (f.alice.clone(), f.bob.clone(), transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
fn spender_transfer_approval_tuple_is_spender_from_to_payload() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (spender.clone(), f.alice.clone(), f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_other_amount_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (f.alice.clone(), f.alice.clone(), 99i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_other_recipient_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (f.alice.clone(), f.bob.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_other_payload_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let mut other = withdraw_payload(e);
    other.c_spend_new = g_times(e, 2);
    let args: Vec<Val> = (f.alice.clone(), f.alice.clone(), 100i128, other).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_without_payload_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (f.alice.clone(), f.alice.clone(), 100i128).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_spender_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> =
        (other, f.alice.clone(), f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_owner_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> =
        (spender.clone(), other, f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_recipient_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> =
        (spender.clone(), f.alice.clone(), other, spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_payload_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let mut other = spender_transfer_payload(e);
    other.c_transfer = g_times(e, 2);
    let args: Vec<Val> = (spender.clone(), f.alice.clone(), f.bob.clone(), other).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_for_other_sender_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> = (other, f.bob.clone(), transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_for_other_recipient_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> = (f.alice.clone(), other, transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_for_other_depositor_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let other = Address::generate(e);
    let args: Vec<Val> = (other, f.bob.clone(), 100i128).into_val(e);
    deposit_with(&f, 100, Some(args));
}

#[test]
fn set_spender_merge_and_revoke_need_no_approval() {
    let f = approval_flow();
    let e = &f.h.e;
    let spender = account(&f.h, AUDITOR);
    let token = ConfidentialTokenClient::new(e, &f.h.host);
    let data = set_spender_data(e);

    let set = invoke(
        &f.h.host,
        "set_spender",
        (f.alice.clone(), spender.clone(), 1_000u32, data.clone()).into_val(e),
        &[],
    );
    token
        .mock_auths(&[MockAuth { address: &f.alice, invoke: &set }])
        .set_spender(&f.alice, &spender, &1_000, &data);

    let merge = invoke(&f.h.host, "merge", (f.alice.clone(),).into_val(e), &[]);
    token.mock_auths(&[MockAuth { address: &f.alice, invoke: &merge }]).merge(&f.alice);

    let revoke =
        invoke(&f.h.host, "revoke_spender", (f.alice.clone(), spender.clone()).into_val(e), &[]);
    token
        .mock_auths(&[MockAuth { address: &f.alice, invoke: &revoke }])
        .revoke_spender(&f.alice, &spender);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_sender() {
    let h = setup();
    let alice = Address::generate(&h.e);
    configure(&h, Some(alice.clone()));
    store_account(&h, &alice, AUDITOR, 1_000, 0);
    let bob = account(&h, AUDITOR);
    run_hook(&h, 4, &alice, &bob);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_depositor() {
    let h = setup();
    let alice = Address::generate(&h.e);
    configure(&h, Some(alice.clone()));
    let bob = account(&h, AUDITOR);
    run_hook(&h, 1, &alice, &bob);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_withdrawer() {
    let h = setup();
    let alice = Address::generate(&h.e);
    configure(&h, Some(alice.clone()));
    store_account(&h, &alice, AUDITOR, 1_000, 0);
    // A distinct recipient, so the rule is triggered by `from` alone.
    let other = Address::generate(&h.e);
    run_hook(&h, 3, &alice, &other);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_spender() {
    let h = setup();
    let spender = Address::generate(&h.e);
    configure(&h, Some(spender.clone()));
    store_account(&h, &spender, AUDITOR, 0, 0);
    let owner = account(&h, AUDITOR);
    let to = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(&h.e, &spender, &owner, &to, &spender_transfer_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let alice = account(&h, AUDITOR);
    run_hook(&h, 4, &alice, &approver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_deposit_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let depositor = Address::generate(&h.e);
    run_hook(&h, 1, &depositor, &approver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_withdraw_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    let alice = account(&h, AUDITOR);
    run_hook(&h, 3, &alice, &approver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_delegating_owner() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let to = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(
            &h.e,
            &spender,
            &approver,
            &to,
            &spender_transfer_payload(&h.e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn approver_cannot_be_the_spender_transfer_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let owner = account(&h, AUDITOR);
    let spender = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(
            &h.e,
            &spender,
            &owner,
            &approver,
            &spender_transfer_payload(&h.e),
        );
    });
}

#[test]
fn approver_may_take_part_in_operations_outside_the_gate() {
    // `set_spender`, `merge` and `revoke_spender` do not consult the approver.
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let other = account(&h, AUDITOR);
    run_hook(&h, 6, &approver, &other);
    run_hook(&h, 2, &approver, &other);
    run_hook(&h, 7, &approver, &other);
}

#[test]
fn rejected_hook_leaves_commitments_unchanged() {
    let h = configured(None);
    let alice = Address::generate(&h.e);
    store_account(&h, &alice, AUDITOR, 1_000, 0);
    let frozen = account(&h, AUDITOR);
    freeze_account(&h, &frozen);
    let token = ConfidentialTokenClient::new(&h.e, &h.host);

    let res = token.try_confidential_transfer(&alice, &frozen, &transfer_data(&h.e));
    assert!(res.is_err());
    h.e.as_contract(&h.host, || {
        assert_eq!(get_account(&h.e, &alice).spendable_commitment, g_times(&h.e, 1_000));
        assert_eq!(get_account(&h.e, &frozen).receiving_commitment, Grumpkin::identity(&h.e));
    });
}

/// Runs `withdraw(alice, recipient, 100, data)` with the sender's
/// authorization and an approver authorization over `approver_args`.
fn withdraw_to_with(f: &Flow, recipient: &Address, approver_args: Vec<Val>) {
    let e = &f.h.e;
    let data = withdraw_data(e);
    let sender = invoke(
        &f.h.host,
        "withdraw",
        (f.alice.clone(), recipient.clone(), 100i128, data.clone()).into_val(e),
        &[],
    );
    let approval = invoke(&f.h.host, "withdraw", approver_args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &f.alice, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .withdraw(&f.alice, recipient, &100, &data);
}

#[test]
fn withdraw_to_another_address_binds_from_then_to() {
    let f = approval_flow();
    let e = &f.h.e;
    let carol = Address::generate(e);
    let args: Vec<Val> = (f.alice.clone(), carol.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_to_with(&f, &carol, args);
    assert_eq!(StellarAssetClient::new(e, &f.h.sac_addr).balance(&carol), 100);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_with_from_and_to_swapped_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let carol = Address::generate(e);
    let args: Vec<Val> = (carol.clone(), f.alice.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_to_with(&f, &carol, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_with_from_and_to_swapped_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (f.bob.clone(), f.alice.clone(), 100i128).into_val(e);
    deposit_with(&f, 100, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_with_from_and_to_swapped_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (f.bob.clone(), f.alice.clone(), transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_with_spender_and_owner_swapped_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (f.alice.clone(), spender.clone(), f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

// ################## BRANCH COVERAGE ##################
//
// Tests derived from a branching-tree pass over the RWA profile: one test
// per branch the tests above leave uncovered.

// ################## BRANCH COVERAGE HELPERS ##################

fn err(code: ComplianceError) -> Result<(), Error> {
    Err(Error::from_contract_error(code as u32))
}

/// Runs `f` inside the token's frame and returns the error it aborts with.
/// Unlike `hook_panic`, the result is a typed `Error`, and a contract error
/// is told apart from a host trap.
fn try_in_host(h: &Harness, f: impl FnOnce()) -> Result<(), Error> {
    h.e.try_as_contract::<(), Error>(&h.host, f).map_err(|r| r.unwrap())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Register,
    Deposit,
    Merge,
    Withdraw,
    Transfer,
    SpenderTransfer,
    SetSpender,
    Revoke,
}

const OPS: [Op; 8] = [
    Op::Register,
    Op::Deposit,
    Op::Merge,
    Op::Withdraw,
    Op::Transfer,
    Op::SpenderTransfer,
    Op::SetSpender,
    Op::Revoke,
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    From,
    Spender,
}

/// Three distinct parties, so the spender never doubles as the recipient
/// (as it does in `run_hook`).
struct Parties {
    from: Address,
    to: Address,
    spender: Address,
}

fn parties(h: &Harness) -> Parties {
    Parties { from: account(h, AUDITOR), to: account(h, AUDITOR), spender: account(h, AUDITOR) }
}

impl Parties {
    fn get(&self, role: Role) -> &Address {
        match role {
            Role::From => &self.from,
            Role::Spender => &self.spender,
        }
    }
}

fn try_hook(h: &Harness, op: Op, p: &Parties) -> Result<(), Error> {
    let e = &h.e;
    try_in_host(h, || match op {
        Op::Register => RwaHooks::on_register(e, &p.from, AUDITOR, &register_payload(e)),
        Op::Deposit => RwaHooks::on_deposit(e, &p.from, &p.to, 1),
        Op::Merge => RwaHooks::on_merge(e, &p.from),
        Op::Withdraw => RwaHooks::on_withdraw(e, &p.from, &p.to, 1, &withdraw_payload(e)),
        Op::Transfer => RwaHooks::on_transfer(e, &p.from, &p.to, &transfer_payload(e)),
        Op::SpenderTransfer => RwaHooks::on_spender_transfer(
            e,
            &p.spender,
            &p.from,
            &p.to,
            &spender_transfer_payload(e),
        ),
        Op::SetSpender => {
            RwaHooks::on_set_spender(e, &p.from, &p.spender, 100, &set_spender_payload(e))
        }
        Op::Revoke => RwaHooks::on_revoke_spender(e, &p.from, &p.spender),
    })
}

// ################## P1: APPROVAL GATE ##################

/// `withdraw(alice, alice, 100, data)` with the sender's entry and, when
/// given, `approval` (entry point name, arguments) signed by `signer`.
fn gate_withdraw(f: &Flow, signer: &Address, approval: Option<(&str, Vec<Val>)>) {
    let e = &f.h.e;
    let data = withdraw_data(e);
    let sender = invoke(
        &f.h.host,
        "withdraw",
        (f.alice.clone(), f.alice.clone(), 100i128, data.clone()).into_val(e),
        &[],
    );
    let token = ConfidentialTokenClient::new(e, &f.h.host);
    match approval {
        None => token
            .mock_auths(&[MockAuth { address: &f.alice, invoke: &sender }])
            .withdraw(&f.alice, &f.alice, &100, &data),
        Some((fn_name, args)) => {
            let approval = invoke(&f.h.host, fn_name, args, &[]);
            token
                .mock_auths(&[
                    MockAuth { address: &f.alice, invoke: &sender },
                    MockAuth { address: signer, invoke: &approval },
                ])
                .withdraw(&f.alice, &f.alice, &100, &data)
        }
    }
}

fn exact_withdraw_args(f: &Flow) -> Vec<Val> {
    let e = &f.h.e;
    e.as_contract(&f.h.host, || {
        withdraw_approval_args(e, &f.alice, &f.alice, 100, &withdraw_payload(e))
    })
}

// Pins branch: on_withdraw -> require_approval / approver configured, no
//   approver entry / Error(Auth, InvalidAction)
// Verifies: INV-14
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_without_approval_fails() {
    let f = approval_flow();
    gate_withdraw(&f, &f.approver, None);
}

// Pins branch: on_withdraw -> require_approval / the exact tuple signed by
//   an address that is not the approver / Error(Auth, InvalidAction)
// Verifies: INV-14
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_signed_by_another_address_fails() {
    let f = approval_flow();
    let outsider = Address::generate(&f.h.e);
    gate_withdraw(&f, &outsider, Some(("withdraw", exact_withdraw_args(&f))));
}

/// `confidential_transfer_from(spender, alice, bob, data)` with the
/// spender's entry and, when given, `approval` signed by `signer`.
fn gate_spend(f: &Flow, spender: &Address, signer: &Address, approval: Option<(&str, Vec<Val>)>) {
    let e = &f.h.e;
    let data = spender_transfer_data(e);
    let sender = invoke(
        &f.h.host,
        "confidential_transfer_from",
        (spender.clone(), f.alice.clone(), f.bob.clone(), data.clone()).into_val(e),
        &[],
    );
    let token = ConfidentialTokenClient::new(e, &f.h.host);
    match approval {
        None => token
            .mock_auths(&[MockAuth { address: spender, invoke: &sender }])
            .confidential_transfer_from(spender, &f.alice, &f.bob, &data),
        Some((fn_name, args)) => {
            let approval = invoke(&f.h.host, fn_name, args, &[]);
            token
                .mock_auths(&[
                    MockAuth { address: spender, invoke: &sender },
                    MockAuth { address: signer, invoke: &approval },
                ])
                .confidential_transfer_from(spender, &f.alice, &f.bob, &data)
        }
    }
}

// Pins branch: on_spender_transfer -> require_approval / approver
//   configured, no approver entry / Error(Auth, InvalidAction)
// Verifies: INV-14
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_without_approval_fails() {
    let (f, spender) = delegated_flow();
    gate_spend(&f, &spender, &f.approver, None);
}

// Pins branch: on_spender_transfer -> require_approval / the exact tuple
//   signed by an address that is not the approver / Error(Auth, InvalidAction)
// Verifies: INV-14
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_signed_by_another_address_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let outsider = Address::generate(e);
    let args = e.as_contract(&f.h.host, || {
        spender_transfer_approval_args(e, &spender, &f.alice, &f.bob, &spender_transfer_payload(e))
    });
    gate_spend(&f, &spender, &outsider, Some(("confidential_transfer_from", args)));
}

/// `deposit(alice, bob, 100)` with the sender's entry and `approval` signed
/// by `signer`.
fn deposit_with_entry(f: &Flow, signer: &Address, approval: &MockAuthInvoke) {
    let e = &f.h.e;
    let sac_transfer = [invoke(
        &f.h.sac_addr,
        "transfer",
        (f.alice.clone(), f.h.host.clone(), 100i128).into_val(e),
        &[],
    )];
    let sender = invoke(
        &f.h.host,
        "deposit",
        (f.alice.clone(), f.bob.clone(), 100i128).into_val(e),
        &sac_transfer,
    );
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &f.alice, invoke: &sender },
            MockAuth { address: signer, invoke: approval },
        ])
        .deposit(&f.alice, &f.bob, &100);
}

// Pins branch: on_deposit -> require_approval / the exact tuple signed by
//   an address that is not the approver / Error(Auth, InvalidAction)
// Verifies: INV-14
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_signed_by_another_address_fails() {
    let f = approval_flow();
    let outsider = Address::generate(&f.h.e);
    let approval = invoke(&f.h.host, "deposit", current_deposit_args(&f, 100), &[]);
    deposit_with_entry(&f, &outsider, &approval);
}

// Pins branch: on_deposit -> require_approval / the exact tuple authorized
//   for another entry point of the token / Error(Auth, InvalidAction)
// Verifies: INV-18
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_under_another_function_name_fails() {
    let f = approval_flow();
    let approval = invoke(&f.h.host, "withdraw", current_deposit_args(&f, 100), &[]);
    deposit_with_entry(&f, &f.approver, &approval);
}

// Pins branch: on_deposit -> require_approval / the exact tuple authorized
//   for another token contract / Error(Auth, InvalidAction)
// Verifies: INV-18
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_for_another_token_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let registry = e.as_contract(&f.h.host, || get_auditor(e));
    let other_token = e.register(RwaHost, (f.h.sac_addr.clone(), f.h.verifier.clone(), registry));
    let approval = invoke(&other_token, "deposit", current_deposit_args(&f, 100), &[]);
    deposit_with_entry(&f, &f.approver, &approval);
}

// Pins branch: on_transfer -> require_approval / the exact tuple
//   authorized for another entry point / Error(Auth, InvalidAction)
// Verifies: INV-18
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_under_another_function_name_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let data = transfer_data(e);
    let sender = invoke(
        &f.h.host,
        "confidential_transfer",
        (f.alice.clone(), f.bob.clone(), data.clone()).into_val(e),
        &[],
    );
    let args = e.as_contract(&f.h.host, || {
        transfer_approval_args(e, &f.alice, &f.bob, &transfer_payload(e))
    });
    let approval = invoke(&f.h.host, "confidential_transfer_from", args, &[]);
    ConfidentialTokenClient::new(e, &f.h.host)
        .mock_auths(&[
            MockAuth { address: &f.alice, invoke: &sender },
            MockAuth { address: &f.approver, invoke: &approval },
        ])
        .confidential_transfer(&f.alice, &f.bob, &data);
}

// Pins branch: require_approval / approver None (disabled after having
//   been set) / deposit and transfer settle with the sender's entry alone
// Verifies: INV-14
#[test]
fn without_an_approver_only_the_sender_authorizes() {
    let f = approval_flow();
    let e = &f.h.e;
    ConfidentialRwaClient::new(e, &f.h.host).set_rwa_config(&AUDITOR, &None, &f.h.admin);

    deposit_with(&f, 100, None);
    assert_eq!(e.auths().len(), 1);
    assert_eq!(e.auths()[0].0, f.alice);

    transfer_with(&f, None);
    assert_eq!(e.auths().len(), 1);
    assert_eq!(e.auths()[0].0, f.alice);
}

// Pins branch: on_register / approver configured / registration needs the
//   account's authorization only
// Verifies: INV-14
#[test]
fn register_needs_no_approval() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver));
    let a = Address::generate(&h.e);
    let data =
        RegisterData { payload: register_payload(&h.e), proof: Bytes::new(&h.e) }.to_xdr(&h.e);
    let call = invoke(&h.host, "register", (a.clone(), AUDITOR, data.clone()).into_val(&h.e), &[]);
    ConfidentialTokenClient::new(&h.e, &h.host)
        .mock_auths(&[MockAuth { address: &a, invoke: &call }])
        .register(&a, &AUDITOR, &data);
    assert_eq!(h.e.auths().len(), 1);
    h.e.as_contract(&h.host, || assert_eq!(get_account(&h.e, &a).auditor_id, AUDITOR));
}

// ################## P2: SAC GATE AND FAIL-CLOSED EXTERNAL CALLS
// ##################

// Pins branch: gate_account -> check_sac / the sender, the owner, the
//   delegating account or the spender is SAC-deauthorized /
//   NotAuthorizedBySac #3603
// Verifies: INV-16
#[test]
fn sac_gate_covers_the_source_and_the_spender() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: true });
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    for (op, role) in [
        (Op::Deposit, Role::From),
        (Op::Withdraw, Role::From),
        (Op::Transfer, Role::From),
        (Op::SpenderTransfer, Role::From),
        (Op::SpenderTransfer, Role::Spender),
        (Op::SetSpender, Role::From),
    ] {
        let p = parties(&h);
        h.sac.set_authorized(p.get(role), &false);
        assert_eq!(
            try_hook(&h, op, &p),
            err(ComplianceError::NotAuthorizedBySac),
            "{op:?} {role:?}"
        );
    }
}

// Pins branch: check_policy / the policy call fails / every hook traps
// Verifies: INV-20
#[test]
fn hooks_trap_when_the_policy_call_fails() {
    let h = setup();
    let failing = h.e.register(PanicPolicy, ());
    h.e.as_contract(&h.host, || {
        set_compliance_config(
            &h.e,
            &ComplianceConfig { policy: Some(failing.clone()), sac_passthrough: false },
        );
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    for op in OPS {
        let p = parties(&h);
        assert!(try_hook(&h, op, &p).is_err(), "{op:?} passed with an unavailable policy");
    }
}

/// A token without the SAC admin interface.
#[contract]
struct PlainToken;

#[contractimpl]
impl PlainToken {
    pub fn decimals(_e: &Env) -> u32 {
        7
    }
}

// Pins branch: check_sac / sac_passthrough over a token with no
//   `authorized` / every hook traps
// Verifies: INV-16, INV-20
#[test]
fn sac_passthrough_over_a_non_sac_underlying_fails_closed() {
    let h = setup();
    let plain = h.e.register(PlainToken, ());
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(&ConfidentialTokenStorageKey::UnderlyingAsset, &plain);
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: true });
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    for op in OPS {
        let p = parties(&h);
        assert!(try_hook(&h, op, &p).is_err(), "{op:?} passed over a non-SAC underlying");
    }
}

// ################## P3: ORDER OF CHECKS IN THE HOOKS ##################

// Pins branch: rwa_context / both configurations absent / every hook
//   reports RwaNotConfigured #3607, not NotConfigured #3600
// Verifies: INV-1
#[test]
fn rwa_config_is_checked_before_compliance_config() {
    let h = setup();
    for op in OPS {
        let p = parties(&h);
        assert_eq!(try_hook(&h, op, &p), err(ComplianceError::RwaNotConfigured), "{op:?}");
    }
}

// Pins branch: RwaHooks::on_* / two conditions fail at once / the earlier
//   check's error is reported
// Verifies: INV-4, INV-5, INV-16
#[test]
fn hook_checks_run_in_documented_order() {
    let h = configured(None);
    let policy = ListPolicyClient::new(&h.e, &h.policy);

    // on_register: auditor id before the policy.
    let a = Address::generate(&h.e);
    policy.block(&a);
    assert_eq!(
        try_in_host(&h, || RwaHooks::on_register(
            &h.e,
            &a,
            LEGACY_AUDITOR,
            &register_payload(&h.e)
        )),
        err(ComplianceError::AuditorNotAllowed)
    );

    // on_transfer: `from` gates before `to` gates.
    let p = parties(&h);
    freeze_account(&h, &p.from);
    policy.block(&p.to);
    assert_eq!(try_hook(&h, Op::Transfer, &p), err(ComplianceError::AccountFrozen));

    // on_spender_transfer: `to` gates before `spender` gates.
    let p = parties(&h);
    policy.block(&p.to);
    freeze_account(&h, &p.spender);
    assert_eq!(try_hook(&h, Op::SpenderTransfer, &p), err(ComplianceError::NotAuthorizedByPolicy));
}

// ################## P4: HOOK VARIANTS ##################

// Pins branch: on_spender_transfer / the spender is bound to another
//   auditor or not registered / the hook passes
// Verifies: INV-5
#[test]
fn spender_binding_and_registration_are_not_checked_on_spend() {
    let h = configured(None);
    for spender in [account(&h, LEGACY_AUDITOR), Address::generate(&h.e)] {
        let p = Parties { from: account(&h, AUDITOR), to: account(&h, AUDITOR), spender };
        assert_eq!(try_hook(&h, Op::SpenderTransfer, &p), Ok(()));
    }
}

// Pins branch: on_register / the address carries a frozen flag / it
//   registers and cannot receive afterwards
// Verifies: INV-16
#[test]
fn frozen_address_can_register_but_not_transact() {
    let h = configured(None);
    let a = Address::generate(&h.e);
    freeze_account(&h, &a);
    assert_eq!(
        try_in_host(&h, || RwaHooks::on_register(&h.e, &a, AUDITOR, &register_payload(&h.e))),
        Ok(())
    );
    store_account(&h, &a, AUDITOR, 0, 0);
    let from = account(&h, AUDITOR);
    assert_eq!(
        try_in_host(&h, || RwaHooks::on_transfer(&h.e, &from, &a, &transfer_payload(&h.e))),
        err(ComplianceError::AccountFrozen)
    );
}

// ################## P5: CONFIGURATION ##################

// Pins branch: set_rwa_config / auditor and approver both invalid / the
//   auditor check wins; the token is rejected as approver on a later call
//   too; a rejected call writes nothing
// Verifies: INV-2, INV-3, INV-8
#[test]
fn set_rwa_config_checks_the_auditor_before_the_approver() {
    let h = setup();
    h.e.as_contract(&h.host, || set_compliance_config(&h.e, &policy_config(&h)));
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    let this = Some(h.host.clone());

    // First call: registry lookup first.
    assert_eq!(
        client.try_set_rwa_config(&7, &this, &h.admin).unwrap_err().unwrap(),
        Error::from_contract_error(3301) // AuditorError::AuditorNotRegistered
    );
    client.set_rwa_config(&AUDITOR, &None, &h.admin);
    // Later calls: the immutable auditor first, then the approver.
    assert_eq!(
        client.try_set_rwa_config(&LEGACY_AUDITOR, &this, &h.admin).unwrap_err().unwrap(),
        Error::from_contract_error(ComplianceError::AuditorAlreadySet as u32)
    );
    assert_eq!(
        client.try_set_rwa_config(&AUDITOR, &this, &h.admin).unwrap_err().unwrap(),
        Error::from_contract_error(ComplianceError::InvalidApprover as u32)
    );
    assert_eq!(client.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR, approver: None }));
}

// Pins branch: set_rwa_config / later call, approver None / exactly one
//   RwaConfigChanged without an approver
// Verifies: INV-22
#[test]
fn disabling_the_approver_emits_rwa_config_changed_without_approver() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver));
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    client.set_rwa_config(&AUDITOR, &None, &h.admin);

    let events = h.e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &RwaConfigChanged { auditor_id: AUDITOR, approver: None }.to_xdr(&h.e, &h.host),
    );
    assert_eq!(client.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR, approver: None }));
}
