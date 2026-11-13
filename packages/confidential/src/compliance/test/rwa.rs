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
            approval_epoch, bump_approval_epoch, can_transact, clawback, compliance_config,
            deposit_approval_args, force_revoke_spender, forced_transfer, freeze, rwa_config,
            set_compliance_config, set_rwa_config, spender_transfer_approval_args,
            transfer_approval_args, unfreeze, withdraw_approval_args,
        },
        ApprovalEpochBumped, ClawbackData, ComplianceConfig, ComplianceConfigChanged,
        ComplianceError, ComplianceStorageKey, ConfidentialClawback, ConfidentialClawbackClient,
        ConfidentialCompliance, ConfidentialComplianceClient, ConfidentialForcedTransfer,
        ConfidentialForcedTransferClient, ConfidentialRwa, ConfidentialRwaClient, ForcedTransfer,
        Policy, RwaConfig, RwaConfigChanged, RwaHooks,
    },
    storage::{
        address_to_field, decode_data, forced_destination_to_field, get_account,
        get_address_as_field_element, get_auditor, set_address_as_field_element, set_auditor,
        set_underlying_asset, set_verifier,
    },
    verifier::CircuitType,
    ConfidentialAccount, ConfidentialToken, ConfidentialTokenClient, ConfidentialTokenError,
    ConfidentialTokenStorageKey, Hooks, RegisterData, RegisterPayload, SetSpenderData,
    SetSpenderPayload, SpenderDelegation, SpenderTransferData, SpenderTransferPayload,
    TransferData, TransferPayload, WithdrawData, WithdrawPayload,
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
impl ConfidentialClawback for RwaHost {
    fn clawback(
        e: &Env,
        account: Address,
        amount: i128,
        destination: Option<Address>,
        data: Bytes,
        admin: Address,
    ) {
        admin.require_auth();
        let d: ClawbackData = decode_data(e, &data);
        clawback(e, &account, amount, &destination, &d.proof);
    }

    fn force_revoke_spender(e: &Env, account: Address, spender: Address, admin: Address) {
        admin.require_auth();
        force_revoke_spender(e, &account, &spender);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialRwa for RwaHost {
    fn set_rwa_config(e: &Env, auditor_id: u32, approver: Option<Address>, admin: Address) {
        admin.require_auth();
        set_rwa_config(e, auditor_id, &approver);
    }

    fn bump_approval_epoch(e: &Env, admin: Address) {
        admin.require_auth();
        bump_approval_epoch(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialForcedTransfer for RwaHost {
    fn forced_transfer(
        e: &Env,
        from: Address,
        to: Address,
        amount: i128,
        data: Bytes,
        admin: Address,
    ) {
        admin.require_auth();
        let d: ClawbackData = decode_data(e, &data);
        forced_transfer(e, &from, &to, amount, &d.proof);
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

/// Accepts every proof and records the public inputs of the last
/// verification, so tests can inspect what the contract assembled.
#[contract]
struct RecordingVerifier;

#[contractimpl]
impl RecordingVerifier {
    pub fn last_public_inputs(e: &Env) -> Bytes {
        e.storage().instance().get(&symbol_short!("pi")).unwrap()
    }

    pub fn last_circuit(e: &Env) -> CircuitType {
        e.storage().instance().get(&symbol_short!("ct")).unwrap()
    }
}

#[contractimpl(contracttrait)]
impl crate::verifier::ConfidentialVerifier for RecordingVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(e: &Env, ct: CircuitType, pi: Bytes, _proof: Bytes) -> bool {
        e.storage().instance().set(&symbol_short!("pi"), &pi);
        e.storage().instance().set(&symbol_short!("ct"), &ct);
        true
    }
}

/// Models a proof's binding to its public inputs: the first blob it accepts
/// becomes the statement, and every later verification succeeds only on the
/// identical blob.
#[contract]
struct BoundProofVerifier;

#[contractimpl(contracttrait)]
impl crate::verifier::ConfidentialVerifier for BoundProofVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(e: &Env, _ct: CircuitType, pi: Bytes, _proof: Bytes) -> bool {
        let key = symbol_short!("bound");
        match e.storage().instance().get::<_, Bytes>(&key) {
            None => {
                e.storage().instance().set(&key, &pi);
                true
            }
            Some(bound) => bound == pi,
        }
    }
}

/// Rejects every proof.
#[contract]
struct RejectingVerifier;

#[contractimpl(contracttrait)]
impl crate::verifier::ConfidentialVerifier for RejectingVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(_e: &Env, _ct: CircuitType, _pi: Bytes, _proof: Bytes) -> bool {
        false
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
    setup_with(|e| e.register(RecordingVerifier, ()))
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

fn proof_data(e: &Env) -> Bytes {
    ClawbackData { proof: Bytes::new(e) }.to_xdr(e)
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
#[should_panic(expected = "Error(Contract, #3615)")]
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
#[should_panic(expected = "Error(Contract, #3610)")]
fn set_rwa_config_rejects_auditor_change() {
    let h = configured(None);
    h.e.as_contract(&h.host, || set_rwa_config(&h.e, LEGACY_AUDITOR, &None));
}

#[test]
#[should_panic(expected = "Error(Contract, #3611)")]
fn set_rwa_config_rejects_token_as_approver() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &policy_config(&h));
        set_rwa_config(&h.e, AUDITOR, &Some(h.host.clone()));
    });
}

#[test]
fn set_rwa_config_stores_emits_and_advances_epoch() {
    let h = setup();
    let approver = Address::generate(&h.e);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    ConfidentialComplianceClient::new(&h.e, &h.host)
        .set_compliance_config(&policy_config(&h), &h.admin);
    assert_eq!(client.rwa_config(), None);
    assert_eq!(client.approval_epoch(), 0);

    client.set_rwa_config(&AUDITOR, &Some(approver.clone()), &h.admin);

    let events = h.e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &RwaConfigChanged { auditor_id: AUDITOR, approver: Some(approver.clone()), epoch: 1 }
            .to_xdr(&h.e, &h.host),
    );
    assert_eq!(
        client.rwa_config(),
        Some(RwaConfig { auditor_id: AUDITOR, approver: Some(approver) })
    );
    assert_eq!(client.approval_epoch(), 1);
}

#[test]
fn approver_can_be_changed_and_disabled_and_epoch_never_repeats() {
    let h = configured(None);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    let a = Address::generate(&h.e);
    assert_eq!(client.approval_epoch(), 1);

    client.set_rwa_config(&AUDITOR, &Some(a.clone()), &h.admin);
    assert_eq!(client.approval_epoch(), 2);
    client.set_rwa_config(&AUDITOR, &None, &h.admin);
    assert_eq!(client.approval_epoch(), 3);
    // Re-enabling the same approver does not return to the epoch it was
    // enabled under before.
    client.set_rwa_config(&AUDITOR, &Some(a), &h.admin);
    assert_eq!(client.approval_epoch(), 4);
}

#[test]
#[should_panic(expected = "Error(Contract, #3607)")]
fn bump_approval_epoch_requires_rwa_config() {
    let h = setup();
    h.e.as_contract(&h.host, || bump_approval_epoch(&h.e));
}

#[test]
fn bump_approval_epoch_increments_and_emits() {
    let h = configured(None);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    client.bump_approval_epoch(&h.admin);

    let events = h.e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &ApprovalEpochBumped { epoch: 2 }.to_xdr(&h.e, &h.host),
    );
    assert_eq!(client.approval_epoch(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3616)")]
fn approval_epoch_overflow_panics() {
    let h = configured(None);
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(&ComplianceStorageKey::ApprovalEpoch, &u32::MAX);
        bump_approval_epoch(&h.e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3615)")]
fn set_compliance_config_cannot_remove_eligibility_gate_under_rwa() {
    let h = configured(None);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: false });
    });
}

#[test]
fn set_compliance_config_advances_epoch_under_rwa() {
    let h = configured(None);
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);
    let config = ComplianceConfig { policy: Some(h.policy.clone()), sac_passthrough: true };
    client.set_compliance_config(&config, &h.admin);

    let events = h.e.events().all();
    assert_eq!(events.events().len(), 2);
    assert_eq!(
        events.events().first().unwrap(),
        &ComplianceConfigChanged { policy: Some(h.policy.clone()), sac_passthrough: true }
            .to_xdr(&h.e, &h.host),
    );
    assert_eq!(
        events.events().get(1).unwrap(),
        &ApprovalEpochBumped { epoch: 2 }.to_xdr(&h.e, &h.host),
    );
    h.e.as_contract(&h.host, || assert_eq!(approval_epoch(&h.e), 2));
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
        assert_eq!(approval_epoch(&h.e), 0);
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
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(
            &ComplianceStorageKey::Config,
            &ComplianceConfig { policy: None, sac_passthrough: false },
        );
    });
    assert_every_hook_reverts_with(&h, &a, &b, 3615);
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
#[should_panic(expected = "Error(Contract, #3615)")]
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

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn on_transfer_rejects_legacy_sender() {
    let h = configured(None);
    let from = account(&h, LEGACY_AUDITOR);
    let to = account(&h, AUDITOR);
    run_hook(&h, 4, &from, &to);
}

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn on_transfer_rejects_legacy_recipient() {
    let h = configured(None);
    let from = account(&h, AUDITOR);
    let to = account(&h, LEGACY_AUDITOR);
    run_hook(&h, 4, &from, &to);
}

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn on_spender_transfer_rejects_legacy_owner() {
    let h = configured(None);
    let owner = account(&h, LEGACY_AUDITOR);
    let to = account(&h, AUDITOR);
    run_hook(&h, 5, &owner, &to);
}

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn on_deposit_rejects_legacy_recipient() {
    let h = configured(None);
    let from = Address::generate(&h.e);
    let to = account(&h, LEGACY_AUDITOR);
    run_hook(&h, 1, &from, &to);
}

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn on_set_spender_rejects_legacy_owner() {
    let h = configured(None);
    let owner = account(&h, LEGACY_AUDITOR);
    let spender = account(&h, AUDITOR);
    run_hook(&h, 6, &owner, &spender);
}

#[test]
fn legacy_account_can_merge_revoke_and_withdraw() {
    let h = configured(None);
    let legacy = account(&h, LEGACY_AUDITOR);
    let other = account(&h, AUDITOR);
    run_hook(&h, 2, &legacy, &other);
    run_hook(&h, 7, &legacy, &other);
    run_hook(&h, 3, &legacy, &other);
}

#[test]
fn spender_auditor_binding_is_not_checked() {
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let legacy_spender = account(&h, LEGACY_AUDITOR);
    run_hook(&h, 6, &owner, &legacy_spender);
}

// ################## ELIGIBILITY GATES ##################

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn on_deposit_rejects_unregistered_recipient() {
    let h = configured(None);
    let from = Address::generate(&h.e);
    let to = Address::generate(&h.e);
    run_hook(&h, 1, &from, &to);
}

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
#[should_panic(expected = "Error(Contract, #3609)")]
fn on_spender_transfer_rejects_legacy_recipient() {
    let h = configured(None);
    let owner = account(&h, AUDITOR);
    let to = account(&h, LEGACY_AUDITOR);
    let spender = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || {
        RwaHooks::on_spender_transfer(&h.e, &spender, &owner, &to, &spender_transfer_payload(&h.e));
    });
}

#[test]
fn can_send_and_can_receive_follow_public_eligibility() {
    let h = configured(None);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    let ok = account(&h, AUDITOR);
    let unregistered = Address::generate(&h.e);
    let legacy = account(&h, LEGACY_AUDITOR);
    let frozen = account(&h, AUDITOR);
    freeze_account(&h, &frozen);
    let denied = account(&h, AUDITOR);
    ListPolicyClient::new(&h.e, &h.policy).block(&denied);

    assert!(client.can_send(&ok));
    assert!(client.can_receive(&ok));
    for a in [&unregistered, &legacy, &frozen, &denied] {
        assert!(!client.can_send(a));
        assert!(!client.can_receive(a));
    }
}

#[test]
fn can_transact_is_false_when_unconfigured() {
    let h = setup();
    let a = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || assert!(!can_transact(&h.e, &a)));

    h.e.as_contract(&h.host, || set_compliance_config(&h.e, &policy_config(&h)));
    h.e.as_contract(&h.host, || assert!(!can_transact(&h.e, &a)));
}

#[test]
fn can_transact_is_false_when_policy_call_fails() {
    let h = setup();
    let failing = h.e.register(PanicPolicy, ());
    h.e.as_contract(&h.host, || {
        set_compliance_config(
            &h.e,
            &ComplianceConfig { policy: Some(failing.clone()), sac_passthrough: false },
        );
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    let a = account(&h, AUDITOR);
    assert!(!ConfidentialRwaClient::new(&h.e, &h.host).can_send(&a));
}

#[test]
fn can_transact_follows_sac_authorization() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: true });
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    let a = account(&h, AUDITOR);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    assert!(client.can_receive(&a));

    h.sac.set_authorized(&a, &false);
    assert!(!client.can_receive(&a));
}

#[test]
fn can_transact_is_false_when_underlying_asset_is_missing() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: None, sac_passthrough: true });
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    let a = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().remove(&ConfidentialTokenStorageKey::UnderlyingAsset);
    });
    assert!(!ConfidentialRwaClient::new(&h.e, &h.host).can_send(&a));
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
fn approval_does_not_survive_epoch_bump() {
    let f = approval_flow();
    let stale = current_deposit_args(&f, 100);
    ConfidentialRwaClient::new(&f.h.e, &f.h.host).bump_approval_epoch(&f.h.admin);
    deposit_with(&f, 100, Some(stale));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn approval_does_not_survive_disable_and_reenable() {
    let f = approval_flow();
    let stale = current_deposit_args(&f, 100);
    let client = ConfidentialRwaClient::new(&f.h.e, &f.h.host);
    client.set_rwa_config(&AUDITOR, &None, &f.h.admin);
    client.set_rwa_config(&AUDITOR, &Some(f.approver.clone()), &f.h.admin);
    deposit_with(&f, 100, Some(stale));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn approval_does_not_survive_policy_change() {
    let f = approval_flow();
    let stale = current_deposit_args(&f, 100);
    let config = ComplianceConfig { policy: Some(f.h.policy.clone()), sac_passthrough: true };
    ConfidentialComplianceClient::new(&f.h.e, &f.h.host).set_compliance_config(&config, &f.h.admin);
    deposit_with(&f, 100, Some(stale));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_under_another_function_name_fails() {
    // The exact `withdraw` tuple, authorized for `deposit`: only the
    // function name differs from the passing case
    // (`withdraw_approval_tuple_is_epoch_from_to_amount_payload`).
    let f = approval_flow();
    let e = &f.h.e;
    let data = withdraw_data(e);
    let sender = invoke(
        &f.h.host,
        "withdraw",
        (f.alice.clone(), f.alice.clone(), 100i128, data.clone()).into_val(e),
        &[],
    );
    let args: Vec<Val> = (
        ConfidentialRwaClient::new(e, &f.h.host).approval_epoch(),
        f.alice.clone(),
        f.alice.clone(),
        100i128,
        withdraw_payload(e),
    )
        .into_val(e);
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
    // (`spender_transfer_approval_tuple_is_epoch_spender_from_to_payload`).
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
    let args: Vec<Val> = (
        ConfidentialRwaClient::new(e, &f.h.host).approval_epoch(),
        spender.clone(),
        f.alice.clone(),
        f.bob.clone(),
        spender_transfer_payload(e),
    )
        .into_val(e);
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

/// The current approval epoch, read through the public getter.
fn epoch(f: &Flow) -> u32 {
    ConfidentialRwaClient::new(&f.h.e, &f.h.host).approval_epoch()
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
fn deposit_approval_tuple_is_epoch_from_to_amount() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (epoch(&f), f.alice.clone(), f.bob.clone(), 100i128).into_val(e);
    deposit_with(&f, 100, Some(args));
}

#[test]
fn withdraw_approval_tuple_is_epoch_from_to_amount_payload() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), f.alice.clone(), f.alice.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
fn transfer_approval_tuple_is_epoch_from_to_payload() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), f.alice.clone(), f.bob.clone(), transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
fn spender_transfer_approval_tuple_is_epoch_spender_from_to_payload() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), spender.clone(), f.alice.clone(), f.bob.clone(), spender_transfer_payload(e))
            .into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_other_amount_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), f.alice.clone(), f.alice.clone(), 99i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_other_recipient_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), f.alice.clone(), f.bob.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_other_payload_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let mut other = withdraw_payload(e);
    other.c_spend_new = g_times(e, 2);
    let args: Vec<Val> = (epoch(&f), f.alice.clone(), f.alice.clone(), 100i128, other).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_without_payload_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (epoch(&f), f.alice.clone(), f.alice.clone(), 100i128).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_for_stale_epoch_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f) - 1, f.alice.clone(), f.alice.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_with(&f, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_spender_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> =
        (epoch(&f), other, f.alice.clone(), f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_owner_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> =
        (epoch(&f), spender.clone(), other, f.bob.clone(), spender_transfer_payload(e)).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_recipient_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> =
        (epoch(&f), spender.clone(), f.alice.clone(), other, spender_transfer_payload(e))
            .into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_other_payload_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let mut other = spender_transfer_payload(e);
    other.c_transfer = g_times(e, 2);
    let args: Vec<Val> =
        (epoch(&f), spender.clone(), f.alice.clone(), f.bob.clone(), other).into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_for_stale_epoch_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (
        epoch(&f) - 1,
        spender.clone(),
        f.alice.clone(),
        f.bob.clone(),
        spender_transfer_payload(e),
    )
        .into_val(e);
    spender_transfer_with(&f, &spender, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_for_other_sender_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> = (epoch(&f), other, f.bob.clone(), transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_for_other_recipient_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let other = account(&f.h, AUDITOR);
    let args: Vec<Val> = (epoch(&f), f.alice.clone(), other, transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_for_other_depositor_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let other = Address::generate(e);
    let args: Vec<Val> = (epoch(&f), other, f.bob.clone(), 100i128).into_val(e);
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
#[should_panic(expected = "Error(Contract, #3612)")]
fn approver_cannot_be_the_sender() {
    let h = setup();
    let alice = Address::generate(&h.e);
    configure(&h, Some(alice.clone()));
    store_account(&h, &alice, AUDITOR, 1_000, 0);
    let bob = account(&h, AUDITOR);
    run_hook(&h, 4, &alice, &bob);
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
fn approver_cannot_be_the_depositor() {
    let h = setup();
    let alice = Address::generate(&h.e);
    configure(&h, Some(alice.clone()));
    let bob = account(&h, AUDITOR);
    run_hook(&h, 1, &alice, &bob);
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
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
#[should_panic(expected = "Error(Contract, #3612)")]
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
#[should_panic(expected = "Error(Contract, #3612)")]
fn approver_cannot_be_the_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let alice = account(&h, AUDITOR);
    run_hook(&h, 4, &alice, &approver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
fn approver_cannot_be_the_deposit_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    store_account(&h, &approver, AUDITOR, 0, 0);
    let depositor = Address::generate(&h.e);
    run_hook(&h, 1, &depositor, &approver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
fn approver_cannot_be_the_withdraw_recipient() {
    let h = setup();
    let approver = Address::generate(&h.e);
    configure(&h, Some(approver.clone()));
    let alice = account(&h, AUDITOR);
    run_hook(&h, 3, &alice, &approver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3612)")]
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
#[should_panic(expected = "Error(Contract, #3612)")]
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
#[should_panic(expected = "Error(Contract, #3617)")]
fn forced_transfer_nonce_overflow_panics() {
    let f = forced_flow(None);
    let e = &f.h.e;
    e.as_contract(&f.h.host, || {
        e.storage()
            .persistent()
            .set(&ComplianceStorageKey::ClawbackNonce(f.from.clone()), &u32::MAX);
    });
    force(&f, 40);
}

// ################## FORCED TRANSFER ##################

/// A frozen source holding `spendable` + `receiving` and a registered
/// destination, on a configured deployment with a funded pool.
struct Forced<'a> {
    h: Harness<'a>,
    from: Address,
    to: Address,
}

fn forced_flow<'a>(approver: Option<Address>) -> Forced<'a> {
    forced_flow_with(approver, |e| e.register(RecordingVerifier, ()))
}

fn forced_flow_with<'a>(
    approver: Option<Address>,
    register_verifier: impl FnOnce(&Env) -> Address,
) -> Forced<'a> {
    let h = setup_with(register_verifier);
    configure(&h, approver);
    let from = Address::generate(&h.e);
    store_account(&h, &from, AUDITOR, 70, 30);
    freeze_account(&h, &from);
    let to = Address::generate(&h.e);
    store_account(&h, &to, AUDITOR, 5, 7);
    h.sac.mint(&h.host, &1_000);
    Forced { h, from, to }
}

fn force(f: &Forced, amount: i128) {
    ConfidentialForcedTransferClient::new(&f.h.e, &f.h.host).forced_transfer(
        &f.from,
        &f.to,
        &amount,
        &proof_data(&f.h.e),
        &f.h.admin,
    );
}

#[test]
fn forced_transfer_moves_amount_and_conserves_value() {
    let f = forced_flow(None);
    let e = &f.h.e;
    force(&f, 40);

    let events = e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &ForcedTransfer { from: f.from.clone(), to: f.to.clone(), amount: 40, nonce: 0 }
            .to_xdr(e, &f.h.host),
    );

    e.as_contract(&f.h.host, || {
        let src = get_account(e, &f.from);
        // C_spend <- 70G + 30G - 40G, C_receive <- O.
        assert_eq!(src.spendable_commitment, g_times(e, 60));
        assert_eq!(src.receiving_commitment, Grumpkin::identity(e));
        let dst = get_account(e, &f.to);
        // Spendable untouched; the credit lands in the receiving balance.
        assert_eq!(dst.spendable_commitment, g_times(e, 5));
        assert_eq!(dst.receiving_commitment, g_times(e, 47));
    });
    // No underlying moved.
    assert_eq!(StellarAssetClient::new(e, &f.h.sac_addr).balance(&f.h.host), 1_000);
    assert_eq!(StellarAssetClient::new(e, &f.h.sac_addr).balance(&f.to), 0);
}

#[test]
fn forced_transfer_can_move_the_full_balance() {
    let f = forced_flow(None);
    let e = &f.h.e;
    force(&f, 100);
    e.as_contract(&f.h.host, || {
        let src = get_account(e, &f.from);
        assert_eq!(src.spendable_commitment, Grumpkin::identity(e));
        assert_eq!(src.receiving_commitment, Grumpkin::identity(e));
        assert_eq!(get_account(e, &f.to).receiving_commitment, g_times(e, 107));
    });
}

#[test]
fn forced_transfer_needs_no_approval() {
    let f = forced_flow(None);
    let e = &f.h.e;
    let approver = Address::generate(e);
    ConfidentialRwaClient::new(e, &f.h.host).set_rwa_config(&AUDITOR, &Some(approver), &f.h.admin);
    let call = invoke(
        &f.h.host,
        "forced_transfer",
        (f.from.clone(), f.to.clone(), 40i128, proof_data(e), f.h.admin.clone()).into_val(e),
        &[],
    );
    // Only the operator authorizes; the approver does not.
    ConfidentialForcedTransferClient::new(e, &f.h.host)
        .mock_auths(&[MockAuth { address: &f.h.admin, invoke: &call }])
        .forced_transfer(&f.from, &f.to, &40, &proof_data(e), &f.h.admin);
    e.as_contract(&f.h.host, || {
        assert_eq!(get_account(e, &f.to).receiving_commitment, g_times(e, 47));
    });
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn forced_transfer_requires_the_operator() {
    let f = forced_flow(None);
    let e = &f.h.e;
    ConfidentialForcedTransferClient::new(e, &f.h.host).mock_auths(&[]).forced_transfer(
        &f.from,
        &f.to,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
fn forced_transfer_shares_and_advances_the_clawback_nonce() {
    let f = forced_flow(None);
    let e = &f.h.e;
    let clawback_client = ConfidentialClawbackClient::new(e, &f.h.host);
    assert_eq!(clawback_client.clawback_nonce(&f.from), 0);

    force(&f, 10);
    assert_eq!(clawback_client.clawback_nonce(&f.from), 1);

    clawback_client.clawback(&f.from, &10, &None, &proof_data(e), &f.h.admin);
    assert_eq!(clawback_client.clawback_nonce(&f.from), 2);

    force(&f, 10);
    let events = e.events().all();
    assert_eq!(clawback_client.clawback_nonce(&f.from), 3);
    assert_eq!(
        events.events().first().unwrap(),
        &ForcedTransfer { from: f.from.clone(), to: f.to.clone(), amount: 10, nonce: 2 }
            .to_xdr(e, &f.h.host),
    );
}

/// Splits a public-input blob into its eleven 32-byte fields.
fn fields(e: &Env, pi: &Bytes) -> std::vec::Vec<BytesN<32>> {
    assert_eq!(pi.len(), 11 * 32);
    (0..11u32)
        .map(|i| {
            let mut buf = [0u8; 32];
            pi.slice(i * 32..(i + 1) * 32).copy_into_slice(&mut buf);
            BytesN::from_array(e, &buf)
        })
        .collect()
}

fn last_public_inputs(h: &Harness) -> std::vec::Vec<BytesN<32>> {
    let pi = RecordingVerifierClient::new(&h.e, &h.verifier).last_public_inputs();
    fields(&h.e, &pi)
}

/// The 64 bytes of a point as its two 32-byte coordinates.
fn coordinates(e: &Env, p: &BytesN<64>) -> (BytesN<32>, BytesN<32>) {
    let bytes = p.to_array();
    let mut x = [0u8; 32];
    let mut y = [0u8; 32];
    x.copy_from_slice(&bytes[..32]);
    y.copy_from_slice(&bytes[32..]);
    (BytesN::from_array(e, &x), BytesN::from_array(e, &y))
}

#[test]
fn forced_transfer_public_inputs_differ_from_clawback_only_in_destination() {
    // One deployment, one source account, one destination, one nonce: the
    // forced transfer is executed, the source is put back to the same
    // commitments and nonce, and the same statement is executed as a
    // clawback. Token (field 7) and source account (field 8) bindings are
    // therefore identical and are compared too.
    let f = forced_flow(None);
    let e = &f.h.e;
    let verifier = RecordingVerifierClient::new(e, &f.h.verifier);

    force(&f, 40);
    let forced_pi = last_public_inputs(&f.h);
    assert_eq!(verifier.last_circuit(), CircuitType::Clawback);

    store_account(&f.h, &f.from, AUDITOR, 70, 30);
    e.as_contract(&f.h.host, || {
        e.storage().persistent().remove(&ComplianceStorageKey::ClawbackNonce(f.from.clone()));
    });
    ConfidentialClawbackClient::new(e, &f.h.host).clawback(
        &f.from,
        &40,
        &Some(f.to.clone()),
        &proof_data(e),
        &f.h.admin,
    );
    let clawback_pi = last_public_inputs(&f.h);
    assert_eq!(verifier.last_circuit(), CircuitType::Clawback);

    for i in (0..11usize).filter(|i| *i != 9) {
        assert_eq!(forced_pi[i].to_array(), clawback_pi[i].to_array(), "field {i}");
    }
    e.as_contract(&f.h.host, || {
        // Fields 0-3: the source's commitments as stored before the call.
        let (sx, sy) = coordinates(e, &g_times(e, 70));
        let (rx, ry) = coordinates(e, &g_times(e, 30));
        assert_eq!([&forced_pi[0], &forced_pi[1]], [&sx, &sy]);
        assert_eq!([&forced_pi[2], &forced_pi[3]], [&rx, &ry]);
        // Fields 4-5: the designated auditor's key.
        let (kx, ky) = coordinates(e, &Grumpkin::generator(e));
        assert_eq!([&forced_pi[4], &forced_pi[5]], [&kx, &ky]);
        // Field 6: the public amount. Field 7: this token. Field 8: the
        // source account. Field 9: the destination binding.
        assert_eq!(forced_pi[6], field(e, 40));
        assert_eq!(forced_pi[7], address_to_field(e, &f.h.host));
        assert_eq!(forced_pi[8], address_to_field(e, &f.from));
        assert_eq!(forced_pi[9], forced_destination_to_field(e, &f.to));
        assert_eq!(clawback_pi[9], address_to_field(e, &f.to));
        assert_ne!(forced_pi[9], clawback_pi[9]);
        assert_ne!(forced_pi[9], BytesN::from_array(e, &[0u8; 32]));
    });
}

#[test]
fn forced_destination_field_is_bound_to_the_destination() {
    let h = setup();
    let a = Address::generate(&h.e);
    let b = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        assert_eq!(forced_destination_to_field(&h.e, &a), forced_destination_to_field(&h.e, &a));
        assert_ne!(forced_destination_to_field(&h.e, &a), forced_destination_to_field(&h.e, &b));
        assert_ne!(forced_destination_to_field(&h.e, &a), address_to_field(&h.e, &a));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3506)")]
fn forced_transfer_proof_does_not_execute_as_clawback() {
    // The verifier binds itself to the public inputs of the first proof it
    // sees (the forced transfer). After the source is restored to the same
    // commitments and nonce, the same statement in clawback mode differs in
    // the destination field and is rejected.
    let f = forced_flow_with(None, |e| e.register(BoundProofVerifier, ()));
    let e = &f.h.e;
    force(&f, 40);
    store_account(&f.h, &f.from, AUDITOR, 70, 30);
    e.as_contract(&f.h.host, || {
        e.storage().persistent().remove(&ComplianceStorageKey::ClawbackNonce(f.from.clone()));
    });
    ConfidentialClawbackClient::new(e, &f.h.host).clawback(
        &f.from,
        &40,
        &Some(f.to.clone()),
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3506)")]
fn forced_transfer_proof_does_not_execute_twice() {
    // Restoring the source's commitments leaves the nonce advanced, so the
    // first proof's statement no longer matches.
    let f = forced_flow_with(None, |e| e.register(BoundProofVerifier, ()));
    force(&f, 40);
    store_account(&f.h, &f.from, AUDITOR, 70, 30);
    force(&f, 40);
}

#[test]
#[should_panic(expected = "Error(Contract, #3506)")]
fn forced_transfer_proof_is_bound_to_the_destination() {
    let f = forced_flow_with(None, |e| e.register(BoundProofVerifier, ()));
    let e = &f.h.e;
    force(&f, 40);
    store_account(&f.h, &f.from, AUDITOR, 70, 30);
    e.as_contract(&f.h.host, || {
        e.storage().persistent().remove(&ComplianceStorageKey::ClawbackNonce(f.from.clone()));
    });
    let other = account(&f.h, AUDITOR);
    ConfidentialForcedTransferClient::new(e, &f.h.host).forced_transfer(
        &f.from,
        &other,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3607)")]
fn forced_transfer_requires_rwa_config() {
    let h = setup();
    h.e.as_contract(&h.host, || set_compliance_config(&h.e, &policy_config(&h)));
    let from = account(&h, AUDITOR);
    freeze_account(&h, &from);
    let to = account(&h, AUDITOR);
    h.e.as_contract(&h.host, || forced_transfer(&h.e, &from, &to, 1, &Bytes::new(&h.e)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3604)")]
fn forced_transfer_requires_frozen_source() {
    let f = forced_flow(None);
    f.h.e.as_contract(&f.h.host, || unfreeze(&f.h.e, &f.from));
    force(&f, 40);
}

#[test]
#[should_panic(expected = "Error(Contract, #3613)")]
fn forced_transfer_zero_amount_panics() {
    let f = forced_flow(None);
    force(&f, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3613)")]
fn forced_transfer_negative_amount_panics() {
    let f = forced_flow(None);
    force(&f, -1);
}

#[test]
#[should_panic(expected = "Error(Contract, #3614)")]
fn forced_transfer_to_self_panics() {
    let f = forced_flow(None);
    let e = &f.h.e;
    ConfidentialForcedTransferClient::new(e, &f.h.host).forced_transfer(
        &f.from,
        &f.from,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3614)")]
fn forced_transfer_to_the_token_panics() {
    let f = forced_flow(None);
    let e = &f.h.e;
    ConfidentialForcedTransferClient::new(e, &f.h.host).forced_transfer(
        &f.from,
        &f.h.host,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn forced_transfer_to_unregistered_account_panics() {
    let f = forced_flow(None);
    let e = &f.h.e;
    let nobody = Address::generate(e);
    ConfidentialForcedTransferClient::new(e, &f.h.host).forced_transfer(
        &f.from,
        &nobody,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn forced_transfer_to_frozen_account_panics() {
    let f = forced_flow(None);
    freeze_account(&f.h, &f.to);
    force(&f, 40);
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn forced_transfer_to_policy_denied_account_panics() {
    let f = forced_flow(None);
    ListPolicyClient::new(&f.h.e, &f.h.policy).block(&f.to);
    force(&f, 40);
}

#[test]
#[should_panic(expected = "Error(Contract, #3609)")]
fn forced_transfer_to_legacy_account_panics() {
    let f = forced_flow(None);
    let e = &f.h.e;
    let legacy = account(&f.h, LEGACY_AUDITOR);
    ConfidentialForcedTransferClient::new(e, &f.h.host).forced_transfer(
        &f.from,
        &legacy,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
}

#[test]
fn forced_transfer_from_legacy_source_uses_its_own_auditor_key() {
    let h = setup();
    configure(&h, None);
    let from = Address::generate(&h.e);
    store_account(&h, &from, LEGACY_AUDITOR, 50, 0);
    freeze_account(&h, &from);
    let to = account(&h, AUDITOR);
    ConfidentialForcedTransferClient::new(&h.e, &h.host).forced_transfer(
        &from,
        &to,
        &50,
        &proof_data(&h.e),
        &h.admin,
    );

    // The proof is verified against the key of the auditor the source is
    // bound to, not the designated auditor's.
    let pi = last_public_inputs(&h);
    let (kx, ky) = coordinates(&h.e, &g_times(&h.e, 2));
    assert_eq!([&pi[4], &pi[5]], [&kx, &ky]);
    h.e.as_contract(&h.host, || {
        assert_eq!(get_account(&h.e, &to).receiving_commitment, g_times(&h.e, 50));
    });
}

#[test]
fn rejected_forced_transfer_proof_changes_nothing() {
    let f = forced_flow_with(None, |e| e.register(RejectingVerifier, ()));
    let e = &f.h.e;
    let res = ConfidentialForcedTransferClient::new(e, &f.h.host).try_forced_transfer(
        &f.from,
        &f.to,
        &40,
        &proof_data(e),
        &f.h.admin,
    );
    assert!(res.is_err());

    e.as_contract(&f.h.host, || {
        let src = get_account(e, &f.from);
        assert_eq!(src.spendable_commitment, g_times(e, 70));
        assert_eq!(src.receiving_commitment, g_times(e, 30));
        assert_eq!(get_account(e, &f.to).receiving_commitment, g_times(e, 7));
    });
    assert_eq!(ConfidentialClawbackClient::new(e, &f.h.host).clawback_nonce(&f.from), 0);
}

#[test]
fn recovery_revokes_delegations_then_moves_the_full_balance() {
    // Freeze, fold every delegation back, then force-transfer everything to
    // a fresh account.
    let f = forced_flow(None);
    let e = &f.h.e;
    let spender = account(&f.h, AUDITOR);
    store_delegation(&f.h, &f.from, &spender, 25);

    ConfidentialClawbackClient::new(e, &f.h.host)
        .force_revoke_spender(&f.from, &spender, &f.h.admin);
    let fresh = account(&f.h, AUDITOR);
    ConfidentialForcedTransferClient::new(e, &f.h.host).forced_transfer(
        &f.from,
        &fresh,
        &125,
        &proof_data(e),
        &f.h.admin,
    );

    e.as_contract(&f.h.host, || {
        let src = get_account(e, &f.from);
        assert_eq!(src.spendable_commitment, Grumpkin::identity(e));
        assert_eq!(src.receiving_commitment, Grumpkin::identity(e));
        assert_eq!(get_account(e, &fresh).receiving_commitment, g_times(e, 125));
    });
}

#[test]
fn rejected_hook_leaves_commitments_unchanged() {
    let h = configured(None);
    let alice = Address::generate(&h.e);
    store_account(&h, &alice, AUDITOR, 1_000, 0);
    let legacy = account(&h, LEGACY_AUDITOR);
    let token = ConfidentialTokenClient::new(&h.e, &h.host);

    let res = token.try_confidential_transfer(&alice, &legacy, &transfer_data(&h.e));
    assert!(res.is_err());
    h.e.as_contract(&h.host, || {
        assert_eq!(get_account(&h.e, &alice).spendable_commitment, g_times(&h.e, 1_000));
        assert_eq!(get_account(&h.e, &legacy).receiving_commitment, Grumpkin::identity(&h.e));
    });
}

fn hex32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

#[test]
fn forced_destination_to_field_matches_testdata_vectors() {
    let h = setup();
    let e = &h.e;

    // (strkey, expected) from
    // circuits/lib/testdata/forced_destination_to_field.json: the two
    // addresses of the `address_to_field` vectors.
    let vectors: [(&str, [u8; 32]); 2] = [
        (
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            hex32("2b07bfd7c614007acad5b1d37a3fdc64a915b44dd0dafc5d5cf0bd2d2ef3ab7a"),
        ),
        (
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
            hex32("2bbf4aadf66599f9f9a99357f94b9c3c2657aa890e277ed9fe366ac4c8f6b5be"),
        ),
    ];

    for (strkey, expected) in vectors {
        let addr = Address::from_str(e, strkey);
        let got = e.as_contract(&h.host, || forced_destination_to_field(e, &addr));
        assert_eq!(got, BytesN::from_array(e, &expected), "forced_destination_to_field({strkey})");
    }
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
    let args: Vec<Val> =
        (epoch(&f), f.alice.clone(), carol.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_to_with(&f, &carol, args);
    assert_eq!(StellarAssetClient::new(e, &f.h.sac_addr).balance(&carol), 100);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn withdraw_approval_with_from_and_to_swapped_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let carol = Address::generate(e);
    let args: Vec<Val> =
        (epoch(&f), carol.clone(), f.alice.clone(), 100i128, withdraw_payload(e)).into_val(e);
    withdraw_to_with(&f, &carol, args);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn deposit_approval_with_from_and_to_swapped_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> = (epoch(&f), f.bob.clone(), f.alice.clone(), 100i128).into_val(e);
    deposit_with(&f, 100, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_approval_with_from_and_to_swapped_fails() {
    let f = approval_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), f.bob.clone(), f.alice.clone(), transfer_payload(e)).into_val(e);
    transfer_with(&f, Some(args));
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn spender_transfer_approval_with_spender_and_owner_swapped_fails() {
    let (f, spender) = delegated_flow();
    let e = &f.h.e;
    let args: Vec<Val> =
        (epoch(&f), f.alice.clone(), spender.clone(), f.bob.clone(), spender_transfer_payload(e))
            .into_val(e);
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

fn token_err(code: ConfidentialTokenError) -> Result<(), Error> {
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

fn try_force(f: &Forced, from: &Address, to: &Address, amount: i128) -> Result<(), Error> {
    ConfidentialForcedTransferClient::new(&f.h.e, &f.h.host)
        .try_forced_transfer(from, to, &amount, &proof_data(&f.h.e), &f.h.admin)
        .map(|r| r.unwrap())
        .map_err(|r| r.unwrap())
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

// Pins branch: can_transact, check_sac / sac_passthrough over a token with
//   no `authorized` / the views return false and every hook traps
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
    let a = account(&h, AUDITOR);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    assert!(!client.can_send(&a));
    assert!(!client.can_receive(&a));
    for op in OPS {
        let p = parties(&h);
        assert!(try_hook(&h, op, &p).is_err(), "{op:?} passed over a non-SAC underlying");
    }
}

// ################## P3: FORCED TRANSFER ##################

// Pins branch: forced_transfer / two preconditions fail at once / the
//   order is freeze, amount, destination identity, source registration,
//   destination gates, destination registration and auditor; state unchanged
// Verifies: INV-6
#[test]
fn forced_transfer_checks_run_in_documented_order() {
    let f = forced_flow(None);
    let h = &f.h;
    let e = &h.e;

    // 1. Freeze before amount, destination and registration.
    let unfrozen = account(h, AUDITOR);
    assert_eq!(try_force(&f, &unfrozen, &unfrozen, 0), err(ComplianceError::AccountNotFrozen));
    let nobody = Address::generate(e);
    assert_eq!(try_force(&f, &nobody, &f.to, 1), err(ComplianceError::AccountNotFrozen));

    // 2. Amount before destination.
    assert_eq!(
        try_force(&f, &f.from, &f.from, 0),
        err(ComplianceError::InvalidForcedTransferAmount)
    );

    // 3. Destination identity before the source's registration.
    let ghost = Address::generate(e);
    freeze_account(h, &ghost);
    assert_eq!(
        try_force(&f, &ghost, &ghost, 1),
        err(ComplianceError::InvalidForcedTransferDestination)
    );
    assert_eq!(
        try_force(&f, &ghost, &h.host, 1),
        err(ComplianceError::InvalidForcedTransferDestination)
    );

    // 4. Source registration before the destination's gates.
    let frozen_to = account(h, AUDITOR);
    freeze_account(h, &frozen_to);
    assert_eq!(
        try_force(&f, &ghost, &frozen_to, 1),
        token_err(ConfidentialTokenError::AccountNotRegistered)
    );

    // 5. Destination gates before its registration and auditor binding.
    let legacy_frozen = account(h, LEGACY_AUDITOR);
    freeze_account(h, &legacy_frozen);
    assert_eq!(try_force(&f, &f.from, &legacy_frozen, 1), err(ComplianceError::AccountFrozen));
    let denied_unregistered = Address::generate(e);
    ListPolicyClient::new(e, &h.policy).block(&denied_unregistered);
    assert_eq!(
        try_force(&f, &f.from, &denied_unregistered, 1),
        err(ComplianceError::NotAuthorizedByPolicy)
    );

    // Nothing moved and the nonce did not advance.
    e.as_contract(&h.host, || {
        assert_eq!(get_account(e, &f.from).spendable_commitment, g_times(e, 70));
        assert_eq!(get_account(e, &f.from).receiving_commitment, g_times(e, 30));
        assert_eq!(get_account(e, &f.to).receiving_commitment, g_times(e, 7));
    });
    assert_eq!(ConfidentialClawbackClient::new(e, &h.host).clawback_nonce(&f.from), 0);
}

// Pins branch: forced_transfer -> gate_account(to) / SAC passthrough, `to`
//   deauthorized / NotAuthorizedBySac #3603; `from` passes no gate
// Verifies: INV-6
#[test]
fn forced_transfer_gates_the_destination_on_the_sac_and_not_the_source() {
    let f = forced_flow(None);
    let e = &f.h.e;
    ConfidentialComplianceClient::new(e, &f.h.host).set_compliance_config(
        &ComplianceConfig { policy: Some(f.h.policy.clone()), sac_passthrough: true },
        &f.h.admin,
    );
    f.h.sac.set_authorized(&f.to, &false);
    assert_eq!(try_force(&f, &f.from, &f.to, 40), err(ComplianceError::NotAuthorizedBySac));

    // The source is not gated: denying it does not block the override.
    f.h.sac.set_authorized(&f.to, &true);
    f.h.sac.set_authorized(&f.from, &false);
    ListPolicyClient::new(e, &f.h.policy).block(&f.from);
    assert_eq!(try_force(&f, &f.from, &f.to, 40), Ok(()));
}

// Pins branch: forced_transfer -> verify / the statement of an executed
//   proof replayed against another account in the same state / InvalidProof
// Verifies: INV-12
#[test]
fn forced_transfer_proof_is_bound_to_the_source_account() {
    let f = forced_flow_with(None, |e| e.register(BoundProofVerifier, ()));
    force(&f, 40);
    // A second frozen account in the state the proof was built against.
    let twin = Address::generate(&f.h.e);
    store_account(&f.h, &twin, AUDITOR, 70, 30);
    freeze_account(&f.h, &twin);
    assert_eq!(try_force(&f, &twin, &f.to, 40), token_err(ConfidentialTokenError::InvalidProof));
}

// Pins branch: forced_transfer -> verify / the statement replayed with
//   another amount / InvalidProof
// Verifies: INV-12
#[test]
fn forced_transfer_proof_is_bound_to_the_amount() {
    let f = forced_flow_with(None, |e| e.register(BoundProofVerifier, ()));
    force(&f, 40);
    store_account(&f.h, &f.from, AUDITOR, 70, 30);
    f.h.e.as_contract(&f.h.host, || {
        f.h.e.storage().persistent().remove(&ComplianceStorageKey::ClawbackNonce(f.from.clone()));
    });
    assert_eq!(try_force(&f, &f.from, &f.to, 41), token_err(ConfidentialTokenError::InvalidProof));
}

// Pins branch: forced_transfer / success / the amount, token and nonce
//   public inputs carry the call's amount, this contract's field element
//   and the stored nonce
// Verifies: INV-11, INV-12
#[test]
fn forced_transfer_public_inputs_carry_amount_token_and_nonce() {
    let f = forced_flow(None);
    let e = &f.h.e;
    e.as_contract(&f.h.host, || {
        e.storage().persistent().set(&ComplianceStorageKey::ClawbackNonce(f.from.clone()), &5u32);
    });
    force(&f, 40);

    let pi = last_public_inputs(&f.h);
    assert_eq!(pi[6], field(e, 40));
    e.as_contract(&f.h.host, || assert_eq!(pi[7], get_address_as_field_element(e)));
    assert_eq!(pi[10], field(e, 5));
}

// Pins branch: forced_transfer -> auditor.get_key / `from` bound to an id
//   the registry does not hold / AuditorNotRegistered #3301
// Verifies: INV-6
#[test]
fn forced_transfer_from_source_with_unregistered_auditor_fails() {
    let f = forced_flow(None);
    let orphan = Address::generate(&f.h.e);
    store_account(&f.h, &orphan, 7, 50, 0);
    freeze_account(&f.h, &orphan);
    // AuditorError::AuditorNotRegistered
    assert_eq!(try_force(&f, &orphan, &f.to, 10), Err(Error::from_contract_error(3301)));
}

// Pins branch: forced_transfer / stored configuration invalid / it reverts
//   with EligibilityRequired #3615 or NotConfigured #3600
// Verifies: INV-1, INV-6
#[test]
fn forced_transfer_fails_closed_on_an_invalid_stored_configuration() {
    let f = forced_flow(None);
    let e = &f.h.e;
    e.as_contract(&f.h.host, || {
        e.storage().instance().set(
            &ComplianceStorageKey::Config,
            &ComplianceConfig { policy: None, sac_passthrough: false },
        );
    });
    assert_eq!(try_force(&f, &f.from, &f.to, 40), err(ComplianceError::EligibilityRequired));
    e.as_contract(&f.h.host, || e.storage().instance().remove(&ComplianceStorageKey::Config));
    assert_eq!(try_force(&f, &f.from, &f.to, 40), err(ComplianceError::NotConfigured));
}

// Pins branch: forced_transfer entry point / `data` is not a ClawbackData /
//   InvalidData #3507
// Verifies: INV-6
#[test]
fn forced_transfer_rejects_malformed_data() {
    let f = forced_flow(None);
    let e = &f.h.e;
    let res = ConfidentialForcedTransferClient::new(e, &f.h.host).try_forced_transfer(
        &f.from,
        &f.to,
        &40,
        &42u32.to_xdr(e),
        &f.h.admin,
    );
    assert_eq!(
        res.unwrap_err().unwrap(),
        Error::from_contract_error(ConfidentialTokenError::InvalidData as u32)
    );
}

// ################## P4: ORDER OF CHECKS IN THE HOOKS ##################

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

    // on_transfer: eligibility gates before the auditor binding.
    let p = Parties {
        from: account(&h, LEGACY_AUDITOR),
        to: account(&h, AUDITOR),
        spender: account(&h, AUDITOR),
    };
    freeze_account(&h, &p.to);
    assert_eq!(try_hook(&h, Op::Transfer, &p), err(ComplianceError::AccountFrozen));

    // on_deposit: policy on `to` before its registration and auditor checks.
    let p = Parties {
        from: Address::generate(&h.e),
        to: Address::generate(&h.e),
        spender: Address::generate(&h.e),
    };
    policy.block(&p.to);
    assert_eq!(try_hook(&h, Op::Deposit, &p), err(ComplianceError::NotAuthorizedByPolicy));

    // on_spender_transfer: `to` gates before `spender` gates.
    let p = parties(&h);
    policy.block(&p.to);
    freeze_account(&h, &p.spender);
    assert_eq!(try_hook(&h, Op::SpenderTransfer, &p), err(ComplianceError::NotAuthorizedByPolicy));
}

// Pins branch: on_transfer / approver == from and from bound to another
//   auditor / AuditorMismatch #3609 wins over ApproverIsParty #3612
// Verifies: INV-3, INV-5
#[test]
fn auditor_binding_is_checked_before_the_approver_is_party_rule() {
    let h = setup();
    let alice = Address::generate(&h.e);
    configure(&h, Some(alice.clone()));
    store_account(&h, &alice, LEGACY_AUDITOR, 0, 0);
    let p = Parties { from: alice, to: account(&h, AUDITOR), spender: account(&h, AUDITOR) };
    assert_eq!(try_hook(&h, Op::Transfer, &p), err(ComplianceError::AuditorMismatch));
}

// ################## P5: HOOK VARIANTS ##################

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

// Pins branch: on_transfer, on_spender_transfer, on_set_spender / a
//   confidential party is not registered / AccountNotRegistered #3501;
//   on_withdraw accepts an unregistered recipient
// Verifies: INV-5, INV-16
#[test]
fn hooks_reject_unregistered_confidential_parties() {
    let h = configured(None);
    let not_registered = token_err(ConfidentialTokenError::AccountNotRegistered);
    let p = Parties {
        from: Address::generate(&h.e),
        to: account(&h, AUDITOR),
        spender: account(&h, AUDITOR),
    };
    assert_eq!(try_hook(&h, Op::Transfer, &p), not_registered);
    assert_eq!(try_hook(&h, Op::SpenderTransfer, &p), not_registered);
    assert_eq!(try_hook(&h, Op::SetSpender, &p), not_registered);
    let p = Parties {
        from: account(&h, AUDITOR),
        to: Address::generate(&h.e),
        spender: account(&h, AUDITOR),
    };
    assert_eq!(try_hook(&h, Op::Transfer, &p), not_registered);
    assert_eq!(try_hook(&h, Op::SpenderTransfer, &p), not_registered);
    // A withdrawal pays out to a public address.
    assert_eq!(try_hook(&h, Op::Withdraw, &p), Ok(()));
}

// Pins branch: on_register / the address carries a frozen flag / it
//   registers; can_receive is false afterwards
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
    assert!(!ConfidentialRwaClient::new(&h.e, &h.host).can_receive(&a));
}

// ################## P6: can_transact ##################

// Pins branch: can_transact / the policy address is not a contract / false
// Verifies: INV-20
#[test]
fn can_transact_is_false_when_the_policy_is_not_a_contract() {
    let h = setup();
    let missing = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(
            &h.e,
            &ComplianceConfig { policy: Some(missing.clone()), sac_passthrough: false },
        );
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    let a = account(&h, AUDITOR);
    assert!(!ConfidentialRwaClient::new(&h.e, &h.host).can_send(&a));
}

// Pins branch: can_transact / policy and SAC both configured / true only
//   when both authorize
// Verifies: INV-16
#[test]
fn can_transact_requires_both_gates_when_both_are_configured() {
    let h = setup();
    h.e.as_contract(&h.host, || {
        set_compliance_config(
            &h.e,
            &ComplianceConfig { policy: Some(h.policy.clone()), sac_passthrough: true },
        );
        set_rwa_config(&h.e, AUDITOR, &None);
    });
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    let ok = account(&h, AUDITOR);
    let denied_by_policy = account(&h, AUDITOR);
    ListPolicyClient::new(&h.e, &h.policy).block(&denied_by_policy);
    let denied_by_sac = account(&h, AUDITOR);
    h.sac.set_authorized(&denied_by_sac, &false);

    assert!(client.can_send(&ok));
    assert!(!client.can_send(&denied_by_policy));
    assert!(!client.can_send(&denied_by_sac));
}

// Pins branch: can_transact / RwaConfig set, ComplianceConfig without a
//   gate or absent / false
// Verifies: INV-16
#[test]
fn can_transact_is_false_for_an_invalid_stored_configuration() {
    let h = configured(None);
    let a = account(&h, AUDITOR);
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    assert!(client.can_send(&a));

    // An eligibility-free configuration, which no setter can write.
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(
            &ComplianceStorageKey::Config,
            &ComplianceConfig { policy: None, sac_passthrough: false },
        );
    });
    assert!(!client.can_send(&a));

    h.e.as_contract(&h.host, || h.e.storage().instance().remove(&ComplianceStorageKey::Config));
    assert!(!client.can_send(&a));
}

// ################## P7: CONFIGURATION ##################

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
    assert_eq!(client.approval_epoch(), 1);
}

// Pins branch: set_rwa_config / later call, approver None / exactly one
//   RwaConfigChanged without an approver, carrying the new epoch
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
        &RwaConfigChanged { auditor_id: AUDITOR, approver: None, epoch: 2 }.to_xdr(&h.e, &h.host),
    );
    assert_eq!(client.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR, approver: None }));
}

// Pins branch: set_compliance_config, set_rwa_config / rejected with
//   EligibilityRequired #3615 or ApprovalEpochOverflow #3616 / the stored
//   configurations and the epoch are unchanged
// Verifies: INV-7, INV-8, INV-9
#[test]
fn rejected_setters_leave_configuration_and_epoch_unchanged() {
    let h = configured(None);
    let rwa = ConfidentialRwaClient::new(&h.e, &h.host);
    let compliance = ConfidentialComplianceClient::new(&h.e, &h.host);
    let open = ComplianceConfig { policy: None, sac_passthrough: false };

    assert_eq!(
        compliance.try_set_compliance_config(&open, &h.admin).unwrap_err().unwrap(),
        Error::from_contract_error(ComplianceError::EligibilityRequired as u32)
    );
    assert_eq!(compliance.compliance_config(), Some(policy_config(&h)));
    assert_eq!(rwa.approval_epoch(), 1);

    // At the last epoch every setter is rejected and nothing is written.
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(&ComplianceStorageKey::ApprovalEpoch, &u32::MAX);
    });
    let overflow = Error::from_contract_error(ComplianceError::ApprovalEpochOverflow as u32);
    let rotated = ComplianceConfig { policy: Some(h.policy.clone()), sac_passthrough: true };
    assert_eq!(
        compliance.try_set_compliance_config(&rotated, &h.admin).unwrap_err().unwrap(),
        overflow
    );
    assert_eq!(compliance.compliance_config(), Some(policy_config(&h)));
    let approver = Address::generate(&h.e);
    assert_eq!(
        rwa.try_set_rwa_config(&AUDITOR, &Some(approver), &h.admin).unwrap_err().unwrap(),
        overflow
    );
    assert_eq!(rwa.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR, approver: None }));
    assert_eq!(rwa.approval_epoch(), u32::MAX);
}

// Pins branch: advance_approval_epoch / epoch == u32::MAX - 1 / the bump
//   succeeds and the epoch reads u32::MAX
// Verifies: INV-7
#[test]
fn approval_epoch_reaches_u32_max() {
    let h = configured(None);
    h.e.as_contract(&h.host, || {
        h.e.storage().instance().set(&ComplianceStorageKey::ApprovalEpoch, &(u32::MAX - 1));
    });
    let client = ConfidentialRwaClient::new(&h.e, &h.host);
    client.bump_approval_epoch(&h.admin);
    assert_eq!(client.approval_epoch(), u32::MAX);
}

// Pins branch: set_compliance_config / RwaConfig set, SAC passthrough as
//   the only gate / stored; epoch advances
// Verifies: INV-8, INV-9
#[test]
fn set_compliance_config_under_rwa_accepts_sac_passthrough_alone() {
    let h = configured(None);
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);
    let sac_only = ComplianceConfig { policy: None, sac_passthrough: true };
    client.set_compliance_config(&sac_only, &h.admin);
    assert_eq!(client.compliance_config(), Some(sac_only));
    assert_eq!(ConfidentialRwaClient::new(&h.e, &h.host).approval_epoch(), 2);
}
