extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    token::StellarAssetClient,
    xdr::{AccountFlags, ToXdr},
    Address, Bytes, BytesN, Env,
};

use crate::confidential::{
    compliance::{
        storage::{
            clawback, compliance_config, force_revoke_spender, freeze, is_frozen,
            set_compliance_config, unfreeze,
        },
        ClawbackData, ComplianceConfig, ComplianceHooks, ComplianceStorageKey,
        ConfidentialClawback, ConfidentialClawbackClient, ConfidentialCompliance,
        ConfidentialComplianceClient, Policy,
    },
    storage::{
        decode_data, set_address_as_field_element, set_auditor, set_underlying_asset, set_verifier,
    },
    verifier::CircuitType,
    ConfidentialAccount, ConfidentialToken, ConfidentialTokenClient, ConfidentialTokenStorageKey,
    Hooks, RegisterData, RegisterPayload, SetSpenderPayload, SpenderDelegation,
    SpenderTransferPayload, TransferPayload, WithdrawPayload,
};

// ################## MOCK CONTRACTS ##################

#[contract]
struct TokenHost;

#[contractimpl]
impl TokenHost {
    pub fn __constructor(e: &Env, token: Address, verifier: Address, auditor: Address) {
        set_underlying_asset(e, &token);
        set_verifier(e, &verifier);
        set_auditor(e, &auditor);
        set_address_as_field_element(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialToken for TokenHost {
    type Hooks = ComplianceHooks;
}

#[contractimpl(contracttrait)]
impl ConfidentialCompliance for TokenHost {
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
impl ConfidentialClawback for TokenHost {
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

#[contract]
struct AllowPolicy;

#[contractimpl]
impl Policy for AllowPolicy {
    fn is_authorized(_e: Env, _account: Address, _token: Address) -> bool {
        true
    }
}

#[contract]
struct DenyPolicy;

#[contractimpl]
impl Policy for DenyPolicy {
    fn is_authorized(_e: Env, _account: Address, _token: Address) -> bool {
        false
    }
}

#[contract]
struct DenyOnePolicy;

#[contractimpl]
impl DenyOnePolicy {
    pub fn __constructor(e: &Env, blocked: Address) {
        e.storage().instance().set(&0u32, &blocked);
    }
}

#[contractimpl]
impl Policy for DenyOnePolicy {
    fn is_authorized(e: Env, account: Address, _token: Address) -> bool {
        let blocked: Address = e.storage().instance().get(&0u32).unwrap();
        account != blocked
    }
}

#[contract]
struct MockVerifier;

#[contractimpl(contracttrait)]
impl crate::confidential::verifier::ConfidentialVerifier for MockVerifier {
    fn register_verification_key(
        _e: &Env,
        _ct: crate::confidential::verifier::CircuitType,
        _verification_key: Bytes,
        _op: Address,
    ) {
    }

    fn update_verification_key(
        _e: &Env,
        _ct: crate::confidential::verifier::CircuitType,
        _verification_key: Bytes,
        _op: Address,
    ) {
    }

    fn verify_proof(
        _e: &Env,
        _ct: crate::confidential::verifier::CircuitType,
        _pi: Bytes,
        _proof: Bytes,
    ) -> bool {
        true
    }
}

#[contract]
struct MockAuditor;

#[contractimpl(contracttrait)]
impl crate::confidential::auditor::ConfidentialAuditor for MockAuditor {
    fn register_key(_e: &Env, _auditor_id: u32, _point: BytesN<64>, _operator: Address) {}

    fn rotate_key(_e: &Env, _auditor_id: u32, _new_point: BytesN<64>, _operator: Address) {}
}

// ################## SETUP ##################

struct Harness<'a> {
    e: Env,
    host: Address,
    sac_addr: Address,
    sac: StellarAssetClient<'a>,
    admin: Address,
}

fn setup<'a>() -> Harness<'a> {
    let e = Env::default();
    e.mock_all_auths();

    let issuer = Address::generate(&e);
    let sac = e.register_stellar_asset_contract_v2(issuer);
    let sac_addr = sac.address();
    // SAC v2 requires the issuer to carry the revocable flag before
    // `set_authorized` is honored.
    sac.issuer().set_flag(AccountFlags::RevocableFlag);
    let sac_client = StellarAssetClient::new(&e, &sac_addr);

    let verifier = e.register(MockVerifier, ());
    let auditor = e.register(MockAuditor, ());
    let host = e.register(TokenHost, (sac_addr.clone(), verifier, auditor));
    let admin = Address::generate(&e);

    Harness { e, host, sac_addr, sac: sac_client, admin }
}

fn base_config() -> ComplianceConfig {
    ComplianceConfig { policy: None, sac_passthrough: false }
}

fn pt(e: &Env) -> BytesN<64> {
    BytesN::from_array(e, &[0u8; 64])
}

fn fr(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[0u8; 32])
}

fn register_payload(e: &Env) -> RegisterPayload {
    RegisterPayload { y: pt(e), pvk: pt(e) }
}

fn withdraw_payload(e: &Env) -> WithdrawPayload {
    WithdrawPayload {
        c_spend_new: pt(e),
        b_tilde: fr(e),
        r_e_point: pt(e),
        sigma: fr(e),
        b_tilde_aud_s: fr(e),
        r_tilde_aud_s: fr(e),
    }
}

fn transfer_payload(e: &Env) -> TransferPayload {
    TransferPayload {
        c_spend_new: pt(e),
        c_transfer: pt(e),
        r_e_point: pt(e),
        v_tilde: fr(e),
        b_tilde: fr(e),
        sigma: fr(e),
        v_tilde_aud_r: fr(e),
        r_tilde_aud_r: fr(e),
        v_tilde_aud_s: fr(e),
        b_tilde_aud_s: fr(e),
        r_tilde_aud_s: fr(e),
    }
}

fn spender_transfer_payload(e: &Env) -> SpenderTransferPayload {
    SpenderTransferPayload {
        c_a_new: pt(e),
        c_transfer: pt(e),
        r_e_point: pt(e),
        v_tilde: fr(e),
        a_tilde_new: fr(e),
        sigma_a_new: fr(e),
        v_tilde_aud_r: fr(e),
        r_tilde_aud_r: fr(e),
        v_tilde_aud_s: fr(e),
        a_tilde_aud_s: fr(e),
        r_tilde_aud_s: fr(e),
    }
}

fn set_spender_payload(e: &Env) -> SetSpenderPayload {
    SetSpenderPayload {
        c_spend_new: pt(e),
        c_a: pt(e),
        escrowed_dvk: pt(e),
        b_tilde: fr(e),
        a_tilde: fr(e),
        r_e_point: pt(e),
        sigma: fr(e),
        sigma_a: fr(e),
        v_tilde_aud_s: fr(e),
        b_tilde_aud_s: fr(e),
        r_tilde_aud_s: fr(e),
        r_a_tilde_aud_s: fr(e),
    }
}

// ################## NO-CONFIG SHORT-CIRCUIT ##################

#[test]
fn hooks_short_circuit_without_config() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        // No config written — every hook must be a silent no-op.
        ComplianceHooks::on_merge(&h.e, &alice);
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, &transfer_payload(&h.e));
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
        ComplianceHooks::on_register(&h.e, &alice, 0, &register_payload(&h.e));
        ComplianceHooks::on_withdraw(&h.e, &alice, &bob, 0, &withdraw_payload(&h.e));
        ComplianceHooks::on_spender_transfer(
            &h.e,
            &op,
            &alice,
            &bob,
            &spender_transfer_payload(&h.e),
        );
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, &set_spender_payload(&h.e));
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op);
        assert!(compliance_config(&h.e).is_none());
        assert!(!is_frozen(&h.e, &alice));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn freeze_without_config_panics_not_configured() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || freeze(&h.e, &alice));
}

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn unfreeze_without_config_panics_not_configured() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || unfreeze(&h.e, &alice));
}

#[test]
fn is_frozen_ignores_stale_flag_when_config_removed() {
    // Freeze alice under an active config, then simulate the config being
    // wiped (e.g. via a storage migration or instance-entry rotation). The
    // persistent Frozen flag survives, but is_frozen must report false
    // because compliance is no longer configured.
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        assert!(is_frozen(&h.e, &alice));

        h.e.storage().instance().remove(&ComplianceStorageKey::Config);
        assert!(compliance_config(&h.e).is_none());
        assert!(!is_frozen(&h.e, &alice));
    });
}

// ################## FREEZE FLOW ##################

#[test]
fn freeze_then_unfreeze_round_trip() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        assert!(!is_frozen(&h.e, &alice));

        freeze(&h.e, &alice);
        assert!(is_frozen(&h.e, &alice));

        unfreeze(&h.e, &alice);
        assert!(!is_frozen(&h.e, &alice));
    });

    // 1 ComplianceConfigChanged + 1 Frozen + 1 Unfrozen.
    assert_eq!(h.e.events().all().events().len(), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_merge_panics_when_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_transfer_panics_when_sender_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, &transfer_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_transfer_panics_when_recipient_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &bob);
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, &transfer_payload(&h.e));
    });
}

#[test]
fn on_spender_transfer_when_spender_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &op);
        ComplianceHooks::on_spender_transfer(
            &h.e,
            &op,
            &alice,
            &bob,
            &spender_transfer_payload(&h.e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_withdraw_panics_when_sender_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        ComplianceHooks::on_withdraw(&h.e, &alice, &bob, 0, &withdraw_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_withdraw_panics_when_recipient_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &bob);
        ComplianceHooks::on_withdraw(&h.e, &alice, &bob, 0, &withdraw_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_set_spender_panics_when_account_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, &set_spender_payload(&h.e));
    });
}

#[test]
fn on_set_spender_when_spender_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &op);
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, &set_spender_payload(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_revoke_spender_panics_when_account_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op);
    });
}

#[test]
fn on_revoke_spender_when_spender_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &op);
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op);
    });
}

#[test]
fn on_register_skips_freeze_check() {
    // Even if a registration entry doesn't exist yet, the user can be
    // "pre-frozen" — on_register intentionally skips the freeze branch.
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, &alice);
        // No panic: register predates the account entry, so the freeze
        // gate is intentionally skipped.
        ComplianceHooks::on_register(&h.e, &alice, 0, &register_payload(&h.e));
    });
}

// ################## POLICY GATE ##################

#[test]
fn passes_with_allowing_policy() {
    let h = setup();
    let policy = h.e.register(AllowPolicy, ());
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn panics_when_policy_denies() {
    let h = setup();
    let policy = h.e.register(DenyPolicy, ());
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
fn rotating_policy_to_none_skips_policy_gate() {
    let h = setup();
    let policy = h.e.register(DenyPolicy, ());
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        // Rotate the policy off; now the deny-everything policy is gone.
        set_compliance_config(&h.e, &base_config());
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_spender_transfer_rejects_policy_denied_spender() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (op.clone(),));
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_spender_transfer(
            &h.e,
            &op,
            &alice,
            &bob,
            &spender_transfer_payload(&h.e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_set_spender_rejects_policy_denied_spender() {
    // Delegating to a policy-denied spender fails at grant time.
    let h = setup();
    let alice = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (op.clone(),));
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, &set_spender_payload(&h.e));
    });
}

#[test]
fn on_revoke_spender_allows_policy_denied_spender() {
    // Revocation is the owner's escape hatch: it must stay possible even
    // after the spender turns non-compliant.
    let h = setup();
    let alice = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (op.clone(),));
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op);
    });
}

// ################## SAC PASSTHROUGH ##################

#[test]
fn passes_when_sac_authorized() {
    // Default behavior of register_stellar_asset_contract_v2: every
    // account is authorized.
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { sac_passthrough: true, ..base_config() });
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3603)")]
fn panics_when_sac_unauthorized() {
    let h = setup();
    let alice = Address::generate(&h.e);
    // Flip the SAC `authorized` flag to false for `alice`.
    h.sac.set_authorized(&alice, &false);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { sac_passthrough: true, ..base_config() });
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
fn on_spender_transfer_when_spender_sac_unauthorized() {
    // The SAC gate targets fund ownership; the spender holds no funds and is
    // intentionally exempt.
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.sac.set_authorized(&op, &false);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { sac_passthrough: true, ..base_config() });
        ComplianceHooks::on_spender_transfer(
            &h.e,
            &op,
            &alice,
            &bob,
            &spender_transfer_payload(&h.e),
        );
    });
}

#[test]
fn sac_passthrough_disabled_skips_sac_call() {
    // With `sac_passthrough=false`, an unauthorized SAC account passes the
    // token-level check.
    let h = setup();
    let alice = Address::generate(&h.e);
    h.sac.set_authorized(&alice, &false);
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &base_config());
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

// ################## DEPOSIT GATING ##################

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_deposit_rejects_policy_denied_from() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_deposit_rejects_policy_denied_to() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    h.e.as_contract(&h.host, || {
        set_compliance_config(&h.e, &ComplianceConfig { policy: Some(policy), ..base_config() });
        ComplianceHooks::on_deposit(&h.e, &bob, &alice, 0);
    });
}

// ################## CONFIG ROTATION ##################

#[test]
fn set_compliance_config_overwrites_atomically() {
    let h = setup();
    let policy_a = h.e.register(AllowPolicy, ());
    let policy_b = h.e.register(DenyPolicy, ());
    h.e.as_contract(&h.host, || {
        set_compliance_config(
            &h.e,
            &ComplianceConfig { policy: Some(policy_a.clone()), sac_passthrough: false },
        );

        let new_config = ComplianceConfig { policy: Some(policy_b.clone()), sac_passthrough: true };
        set_compliance_config(&h.e, &new_config);

        let stored = compliance_config(&h.e).unwrap();
        assert_eq!(stored.policy, Some(policy_b));
        assert!(stored.sac_passthrough);
    });

    // 2 ComplianceConfigChanged events.
    assert_eq!(h.e.events().all().events().len(), 2);
}

// ################## COMBINED GATES ##################

#[test]
fn all_three_gates_pass_together() {
    let h = setup();
    let policy = h.e.register(AllowPolicy, ());
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config(
            &h.e,
            &ComplianceConfig { policy: Some(policy), sac_passthrough: true },
        );
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

// ################## CONFIDENTIAL COMPLIANCE TRAIT (CLIENT API)
// ##################

#[test]
fn trait_set_compliance_config_writes_and_reads_back() {
    let h = setup();
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);

    let config = ComplianceConfig { policy: None, sac_passthrough: true };
    client.set_compliance_config(&config, &h.admin);

    let stored = client.compliance_config().unwrap();
    assert_eq!(stored, config);
}

#[test]
fn trait_freeze_then_unfreeze_via_client() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);
    client.set_compliance_config(&base_config(), &h.admin);

    assert!(!client.is_frozen(&alice));
    client.freeze(&alice, &h.admin);
    assert!(client.is_frozen(&alice));
    client.unfreeze(&alice, &h.admin);
    assert!(!client.is_frozen(&alice));
}

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn trait_freeze_without_config_reverts() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);
    client.freeze(&alice, &h.admin);
}

#[test]
fn trait_is_frozen_returns_false_without_config() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let client = ConfidentialComplianceClient::new(&h.e, &h.host);
    assert!(!client.is_frozen(&alice));
    assert!(client.compliance_config().is_none());
}

// ################## COMPLIANCE HOOKS DISPATCH (END-TO-END) ##################

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn compliance_hooks_blocks_deposit_to_frozen_recipient() {
    // Wires ComplianceHooks via TokenHost::type Hooks; calling deposit
    // through the token client routes the on_deposit callback to the
    // hooks impl, which reverts AccountFrozen on the frozen recipient.
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);
    let admin_client = ConfidentialComplianceClient::new(&h.e, &h.host);

    h.e.as_contract(&h.host, || register_minimal_account(&h.e, &alice));
    admin_client.set_compliance_config(&base_config(), &h.admin);
    admin_client.freeze(&alice, &h.admin);

    h.sac.mint(&depositor, &100);
    let token = ConfidentialTokenClient::new(&h.e, &h.host);
    token.deposit(&depositor, &alice, &50);
}

#[test]
fn compliance_hooks_allows_deposit_without_config() {
    // With ComplianceHooks wired but no config set, the on_deposit hook
    // short-circuits and the deposit succeeds normally.
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.e.as_contract(&h.host, || register_minimal_account(&h.e, &alice));
    h.sac.mint(&depositor, &100);

    let token = ConfidentialTokenClient::new(&h.e, &h.host);
    token.deposit(&depositor, &alice, &50);
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn compliance_hooks_blocks_register_via_policy() {
    // on_register consults the policy gate. With DenyOnePolicy returning
    // false for alice, the register entry point reverts
    // NotAuthorizedByPolicy.
    let h = setup();
    let alice = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    let admin_client = ConfidentialComplianceClient::new(&h.e, &h.host);
    admin_client.set_compliance_config(
        &ComplianceConfig { policy: Some(policy), ..base_config() },
        &h.admin,
    );

    // Trigger on_register via the token client; expect
    // NotAuthorizedByPolicy (#3602).
    let token = ConfidentialTokenClient::new(&h.e, &h.host);
    let register_data = RegisterData {
        payload: RegisterPayload {
            y: BytesN::from_array(&h.e, &[0u8; 64]),
            pvk: BytesN::from_array(&h.e, &[0u8; 64]),
        },
        proof: Bytes::new(&h.e),
    }
    .to_xdr(&h.e);
    token.register(&alice, &1u32, &register_data);
}

#[test]
fn storage_keys_isolated_from_token_keys() {
    // ComplianceStorageKey discriminants do not collide with
    // ConfidentialTokenStorageKey ones: writing the compliance config does not
    // disturb the token's stored SAC address.
    let h = setup();
    h.e.as_contract(&h.host, || {
        let before = crate::confidential::storage::get_underlying_asset(&h.e);
        set_compliance_config(&h.e, &base_config());
        let after = crate::confidential::storage::get_underlying_asset(&h.e);
        assert_eq!(before, after);
        assert_eq!(after, h.sac_addr);
    });
}

// ################## CLAWBACK (SMOKE) ##################
//
// Contract-layer plumbing only. The verifier is mocked, so nothing here
// exercises CB1-CB3, the seize bound, or the destination binding -- those are
// circuit-side (`circuits/clawback/src/tests.nr`). Full coverage of the replay
// and redirect cases needs a public-input-binding verifier mock.

fn clawback_data(e: &Env) -> Bytes {
    ClawbackData { proof: Bytes::new(e) }.to_xdr(e)
}

/// Registers `account` with a spendable commitment of `amount * G` and an
/// empty receiving side, then freezes it.
fn frozen_account_with(h: &Harness, account: &Address, amount: u128) {
    use stellar_contract_utils::crypto::grumpkin::Grumpkin;
    h.e.as_contract(&h.host, || {
        let identity = Grumpkin::identity(&h.e);
        let acc = ConfidentialAccount {
            spending_public_key: identity.clone(),
            viewing_public_key: identity.clone(),
            spendable_commitment: Grumpkin::mul(&h.e, &Grumpkin::generator(&h.e), amount),
            receiving_commitment: identity,
            auditor_id: 0,
        };
        h.e.storage()
            .persistent()
            .set(&ConfidentialTokenStorageKey::Account(account.clone()), &acc);
        set_compliance_config(&h.e, &base_config());
        freeze(&h.e, account);
    });
}

#[test]
fn clawback_none_folds_commitments_and_moves_no_underlying() {
    let h = setup();
    let alice = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);
    h.sac.mint(&h.host, &1_000);

    let client = ConfidentialClawbackClient::new(&h.e, &h.host);
    client.clawback(&alice, &40i128, &None, &clawback_data(&h.e), &h.admin);

    // C_spend <- C_spend + O - 40*G, C_receive <- O.
    h.e.as_contract(&h.host, || {
        use stellar_contract_utils::crypto::grumpkin::Grumpkin;
        let acc = crate::confidential::storage::get_account(&h.e, &alice);
        let expected = Grumpkin::mul(&h.e, &Grumpkin::generator(&h.e), 60);
        assert_eq!(acc.spendable_commitment, expected);
        assert_eq!(acc.receiving_commitment, Grumpkin::identity(&h.e));
    });
    // No underlying moved: the pool is now over-collateralized by 40.
    assert_eq!(StellarAssetClient::new(&h.e, &h.sac_addr).balance(&h.host), 1_000);
    // The freeze survives the seizure.
    h.e.as_contract(&h.host, || assert!(is_frozen(&h.e, &alice)));
}

#[test]
fn clawback_some_transfers_exactly_the_seized_amount() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let dest = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);
    h.sac.mint(&h.host, &1_000);

    let client = ConfidentialClawbackClient::new(&h.e, &h.host);
    client.clawback(&alice, &40i128, &Some(dest.clone()), &clawback_data(&h.e), &h.admin);

    let sac = StellarAssetClient::new(&h.e, &h.sac_addr);
    assert_eq!(sac.balance(&dest), 40);
    assert_eq!(sac.balance(&h.host), 960);
}

#[test]
#[should_panic(expected = "Error(Contract, #3604)")]
fn clawback_unfrozen_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);
    h.e.as_contract(&h.host, || unfreeze(&h.e, &alice));

    ConfidentialClawbackClient::new(&h.e, &h.host).clawback(
        &alice,
        &1i128,
        &None,
        &clawback_data(&h.e),
        &h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3604)")]
fn clawback_freeze_check_precedes_registration_check() {
    // An address that is neither frozen nor registered yields 3604, not 3501.
    let h = setup();
    let nobody = Address::generate(&h.e);
    h.e.as_contract(&h.host, || set_compliance_config(&h.e, &base_config()));

    ConfidentialClawbackClient::new(&h.e, &h.host).clawback(
        &nobody,
        &1i128,
        &None,
        &clawback_data(&h.e),
        &h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3605)")]
fn clawback_zero_amount_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);

    ConfidentialClawbackClient::new(&h.e, &h.host).clawback(
        &alice,
        &0i128,
        &None,
        &clawback_data(&h.e),
        &h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3605)")]
fn clawback_negative_amount_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);

    ConfidentialClawbackClient::new(&h.e, &h.host).clawback(
        &alice,
        &-1i128,
        &None,
        &clawback_data(&h.e),
        &h.admin,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3606)")]
fn clawback_to_self_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);

    ConfidentialClawbackClient::new(&h.e, &h.host).clawback(
        &alice,
        &1i128,
        &Some(h.host.clone()),
        &clawback_data(&h.e),
        &h.admin,
    );
}

#[test]
fn force_revoke_spender_folds_allowance_and_deletes_delegation() {
    use stellar_contract_utils::crypto::grumpkin::Grumpkin;
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);
    h.e.as_contract(&h.host, || {
        h.e.storage().persistent().set(
            &ConfidentialTokenStorageKey::Delegation(alice.clone(), spender.clone()),
            &SpenderDelegation {
                allowance_commitment: Grumpkin::mul(&h.e, &Grumpkin::generator(&h.e), 25),
                a_tilde: fr(&h.e),
                escrowed_dvk: Grumpkin::identity(&h.e),
                allowance_salt: fr(&h.e),
                live_until_ledger: 1_000,
            },
        );
    });

    ConfidentialClawbackClient::new(&h.e, &h.host).force_revoke_spender(&alice, &spender, &h.admin);

    h.e.as_contract(&h.host, || {
        let acc = crate::confidential::storage::get_account(&h.e, &alice);
        assert_eq!(acc.spendable_commitment, Grumpkin::mul(&h.e, &Grumpkin::generator(&h.e), 125));
        assert!(!h
            .e
            .storage()
            .persistent()
            .has(&ConfidentialTokenStorageKey::Delegation(alice.clone(), spender.clone())));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3604)")]
fn force_revoke_spender_unfrozen_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);
    h.e.as_contract(&h.host, || unfreeze(&h.e, &alice));

    ConfidentialClawbackClient::new(&h.e, &h.host).force_revoke_spender(&alice, &spender, &h.admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn force_revoke_unknown_delegation_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    frozen_account_with(&h, &alice, 100);

    ConfidentialClawbackClient::new(&h.e, &h.host).force_revoke_spender(&alice, &spender, &h.admin);
}

// ################## HELPERS ##################

fn register_minimal_account(e: &Env, account: &Address) {
    // Bypass proof verification: the unregistered-deposit tests only need
    // `account_exists` to return true for selected addresses.
    use stellar_contract_utils::crypto::grumpkin::Grumpkin;

    use crate::confidential::{ConfidentialAccount, ConfidentialTokenStorageKey};
    let identity = Grumpkin::identity(e);
    let acc = ConfidentialAccount {
        spending_public_key: identity.clone(),
        viewing_public_key: identity.clone(),
        spendable_commitment: identity.clone(),
        receiving_commitment: identity,
        auditor_id: 0,
    };
    e.storage().persistent().set(&ConfidentialTokenStorageKey::Account(account.clone()), &acc);
}
