extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    token::StellarAssetClient,
    xdr::{AccountFlags, ToXdr},
    Address, Bytes, BytesN, Env, IntoVal, Val,
};
use stellar_event_assertion::EventAssertion;

use crate::confidential::{
    compliance::{
        storage::{compliance_config, freeze, is_frozen, set_compliance_config, unfreeze},
        ComplianceConfig, ComplianceHooks, ConfidentialCompliance, ConfidentialComplianceClient,
        Policy,
    },
    storage::{set_address_as_field_element, set_auditor, set_underlying_asset, set_verifier},
    verifier::CircuitType,
    ConfidentialAccount, ConfidentialToken, ConfidentialTokenClient, Hooks, RegisterData,
    RegisterPayload, SpenderDelegation,
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
        _vk: Bytes,
        _op: Address,
    ) {
    }

    fn update_verification_key(
        _e: &Env,
        _ct: crate::confidential::verifier::CircuitType,
        _vk: Bytes,
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

fn void_val(e: &Env) -> Val {
    ().into_val(e)
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
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, void_val(&h.e));
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
        ComplianceHooks::on_register(&h.e, &alice, void_val(&h.e));
        ComplianceHooks::on_withdraw(&h.e, &alice, &bob, 0, void_val(&h.e));
        ComplianceHooks::on_spender_transfer(&h.e, &op, &alice, &bob, void_val(&h.e));
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, void_val(&h.e));
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op, void_val(&h.e));
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
    EventAssertion::new(&h.e, h.host.clone()).assert_event_count(3);
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
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, void_val(&h.e));
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
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, void_val(&h.e));
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
        ComplianceHooks::on_spender_transfer(&h.e, &op, &alice, &bob, void_val(&h.e));
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
        ComplianceHooks::on_withdraw(&h.e, &alice, &bob, 0, void_val(&h.e));
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
        ComplianceHooks::on_withdraw(&h.e, &alice, &bob, 0, void_val(&h.e));
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
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, void_val(&h.e));
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
        ComplianceHooks::on_set_spender(&h.e, &alice, &op, 0, void_val(&h.e));
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
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op, void_val(&h.e));
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
        ComplianceHooks::on_revoke_spender(&h.e, &alice, &op, void_val(&h.e));
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
        ComplianceHooks::on_register(&h.e, &alice, void_val(&h.e));
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
    EventAssertion::new(&h.e, h.host.clone()).assert_event_count(2);
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

// ################## HELPERS ##################

fn register_minimal_account(e: &Env, account: &Address) {
    // Bypass proof verification: the unregistered-deposit tests only need
    // `account_exists` to return true for selected addresses.
    use stellar_contract_utils::crypto::grumpkin::Grumpkin;

    use crate::confidential::{ConfidentialAccount, ConfidentialTokenStorageKey};
    let identity = Grumpkin::identity(e);
    let acc = ConfidentialAccount {
        spending_key: identity.clone(),
        viewing_public_key: identity.clone(),
        spendable_balance: identity.clone(),
        receiving_balance: identity,
        auditor_id: 0,
    };
    e.storage().persistent().set(&ConfidentialTokenStorageKey::Account(account.clone()), &acc);
}
