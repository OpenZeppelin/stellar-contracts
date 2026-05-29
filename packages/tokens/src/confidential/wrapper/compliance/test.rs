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
    verifier::CircuitType,
    wrapper::{
        compliance::{
            storage::{
                compliance_config, freeze_no_auth, is_frozen, set_compliance_config_no_auth,
                unfreeze_no_auth,
            },
            ComplianceConfig, ComplianceHooks, ConfidentialCompliance,
            ConfidentialComplianceClient, Policy,
        },
        storage::{set_auditor, set_token, set_verifier, set_wrap},
        ConfidentialAccount, ConfidentialTokenWrapper, ConfidentialTokenWrapperClient, Hooks,
        OperatorDelegation, RegisterData, RegisterPayload,
    },
};

// ################## MOCK CONTRACTS ##################

#[contract]
struct WrapperHost;

#[contractimpl]
impl WrapperHost {
    pub fn __constructor(e: &Env, token: Address, verifier: Address, auditor: Address) {
        set_token(e, &token);
        set_verifier(e, &verifier);
        set_auditor(e, &auditor);
        set_wrap(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialTokenWrapper for WrapperHost {
    type Hooks = ComplianceHooks;
}

#[contractimpl(contracttrait)]
impl ConfidentialCompliance for WrapperHost {
    fn freeze(e: &Env, account: Address, admin: Address) {
        admin.require_auth();
        freeze_no_auth(e, &account);
    }

    fn unfreeze(e: &Env, account: Address, admin: Address) {
        admin.require_auth();
        unfreeze_no_auth(e, &account);
    }

    fn set_compliance_config(e: &Env, config: ComplianceConfig, admin: Address) {
        admin.require_auth();
        set_compliance_config_no_auth(e, &config);
    }
}

#[contract]
struct AllowPolicy;

#[contractimpl]
impl Policy for AllowPolicy {
    fn is_authorized(_e: Env, _account: Address, _wrapper: Address) -> bool {
        true
    }
}

#[contract]
struct DenyPolicy;

#[contractimpl]
impl Policy for DenyPolicy {
    fn is_authorized(_e: Env, _account: Address, _wrapper: Address) -> bool {
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
    fn is_authorized(e: Env, account: Address, _wrapper: Address) -> bool {
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
    let host = e.register(WrapperHost, (sac_addr.clone(), verifier, auditor));
    let admin = Address::generate(&e);

    Harness { e, host, sac_addr, sac: sac_client, admin }
}

fn base_config() -> ComplianceConfig {
    ComplianceConfig { policy: None, sac_passthrough: false, permit_unregistered_deposit: false }
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
    h.e.as_contract(&h.host, || {
        // No config written — every hook must be a silent no-op.
        ComplianceHooks::on_merge(&h.e, &alice);
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, void_val(&h.e));
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
        ComplianceHooks::on_register(&h.e, &alice, void_val(&h.e));
        assert!(compliance_config(&h.e).is_none());
        assert!(!is_frozen(&h.e, &alice));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn freeze_without_config_panics_not_configured() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || freeze_no_auth(&h.e, &alice));
}

#[test]
#[should_panic(expected = "Error(Contract, #3600)")]
fn unfreeze_without_config_panics_not_configured() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || unfreeze_no_auth(&h.e, &alice));
}

// ################## FREEZE FLOW ##################

#[test]
fn freeze_then_unfreeze_round_trip() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config_no_auth(&h.e, &base_config());
        assert!(!is_frozen(&h.e, &alice));

        freeze_no_auth(&h.e, &alice);
        assert!(is_frozen(&h.e, &alice));

        unfreeze_no_auth(&h.e, &alice);
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
        set_compliance_config_no_auth(&h.e, &base_config());
        freeze_no_auth(&h.e, &alice);
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
        set_compliance_config_no_auth(&h.e, &base_config());
        freeze_no_auth(&h.e, &alice);
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
        set_compliance_config_no_auth(&h.e, &base_config());
        freeze_no_auth(&h.e, &bob);
        ComplianceHooks::on_transfer(&h.e, &alice, &bob, void_val(&h.e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3601)")]
fn on_operator_transfer_panics_when_operator_frozen() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let op = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config_no_auth(&h.e, &base_config());
        freeze_no_auth(&h.e, &op);
        ComplianceHooks::on_operator_transfer(&h.e, &op, &alice, &bob, void_val(&h.e));
    });
}

#[test]
fn on_register_skips_freeze_check() {
    // Even if a registration slot doesn't exist yet, the user can be
    // "pre-frozen" — on_register intentionally skips the freeze branch.
    let h = setup();
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config_no_auth(&h.e, &base_config());
        freeze_no_auth(&h.e, &alice);
        // No panic: register predates the account slot, so the freeze
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
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig { policy: Some(policy), ..base_config() },
        );
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
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig { policy: Some(policy), ..base_config() },
        );
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
fn rotating_policy_to_none_skips_policy_gate() {
    let h = setup();
    let policy = h.e.register(DenyPolicy, ());
    let alice = Address::generate(&h.e);
    h.e.as_contract(&h.host, || {
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig { policy: Some(policy), ..base_config() },
        );
        // Rotate the policy off; now the deny-everything policy is gone.
        set_compliance_config_no_auth(&h.e, &base_config());
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
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig { sac_passthrough: true, ..base_config() },
        );
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
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig { sac_passthrough: true, ..base_config() },
        );
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

#[test]
fn sac_passthrough_disabled_skips_sac_call() {
    // With `sac_passthrough=false`, an unauthorized SAC account passes the
    // wrapper-level check.
    let h = setup();
    let alice = Address::generate(&h.e);
    h.sac.set_authorized(&alice, &false);
    h.e.as_contract(&h.host, || {
        set_compliance_config_no_auth(&h.e, &base_config());
        ComplianceHooks::on_merge(&h.e, &alice);
    });
}

// ################## UNREGISTERED DEPOSIT CARVE-OUT ##################

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_deposit_rejects_unregistered_from_by_default() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    h.e.as_contract(&h.host, || {
        // Register only `bob` so `alice` exercises the unregistered path.
        register_minimal_account(&h.e, &bob);
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig {
                policy: Some(policy),
                permit_unregistered_deposit: false,
                ..base_config()
            },
        );
        // `permit=false` → unregistered `alice` is still policy-checked
        // and is rejected.
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
    });
}

#[test]
fn on_deposit_permits_unregistered_from_when_flag_set() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    h.e.as_contract(&h.host, || {
        // `alice` is NOT registered → the carve-out applies.
        register_minimal_account(&h.e, &bob);
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig {
                policy: Some(policy),
                permit_unregistered_deposit: true,
                ..base_config()
            },
        );
        // Policy would reject `alice` if consulted; the carve-out
        // suppresses both the freeze and policy checks on `from`.
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_deposit_carveout_does_not_apply_to_registered_from() {
    // When `from` IS registered, the carve-out does not fire and the
    // policy gate runs as usual.
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    h.e.as_contract(&h.host, || {
        register_minimal_account(&h.e, &alice);
        register_minimal_account(&h.e, &bob);
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig {
                policy: Some(policy),
                permit_unregistered_deposit: true,
                ..base_config()
            },
        );
        ComplianceHooks::on_deposit(&h.e, &alice, &bob, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3602)")]
fn on_deposit_to_side_always_checked() {
    // Even with the carve-out enabled and `from` unregistered, the
    // recipient `to` must still pass the policy gate.
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let policy = h.e.register(DenyOnePolicy, (alice.clone(),));
    h.e.as_contract(&h.host, || {
        // `alice` (the policy-rejected address) is the recipient now.
        register_minimal_account(&h.e, &alice);
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig {
                policy: Some(policy),
                permit_unregistered_deposit: true,
                ..base_config()
            },
        );
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
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig {
                policy: Some(policy_a.clone()),
                sac_passthrough: false,
                permit_unregistered_deposit: false,
            },
        );

        let new_config = ComplianceConfig {
            policy: Some(policy_b.clone()),
            sac_passthrough: true,
            permit_unregistered_deposit: true,
        };
        set_compliance_config_no_auth(&h.e, &new_config);

        let stored = compliance_config(&h.e).unwrap();
        assert_eq!(stored.policy, Some(policy_b));
        assert!(stored.sac_passthrough);
        assert!(stored.permit_unregistered_deposit);
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
        set_compliance_config_no_auth(
            &h.e,
            &ComplianceConfig {
                policy: Some(policy),
                sac_passthrough: true,
                permit_unregistered_deposit: false,
            },
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

    let config =
        ComplianceConfig { policy: None, sac_passthrough: true, permit_unregistered_deposit: true };
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
    // Wires ComplianceHooks via WrapperHost::type Hooks; calling deposit
    // through the wrapper client routes the on_deposit callback to the
    // hooks impl, which reverts AccountFrozen on the frozen recipient.
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);
    let admin_client = ConfidentialComplianceClient::new(&h.e, &h.host);

    h.e.as_contract(&h.host, || register_minimal_account(&h.e, &alice));
    admin_client.set_compliance_config(&base_config(), &h.admin);
    admin_client.freeze(&alice, &h.admin);

    h.sac.mint(&depositor, &100);
    let wrapper = ConfidentialTokenWrapperClient::new(&h.e, &h.host);
    wrapper.deposit(&depositor, &alice, &50);
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

    let wrapper = ConfidentialTokenWrapperClient::new(&h.e, &h.host);
    wrapper.deposit(&depositor, &alice, &50);
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

    // Trigger on_register via the wrapper client; expect
    // NotAuthorizedByPolicy (#3602).
    let wrapper = ConfidentialTokenWrapperClient::new(&h.e, &h.host);
    let register_data = RegisterData {
        payload: RegisterPayload {
            y: BytesN::from_array(&h.e, &[0u8; 64]),
            pvk: BytesN::from_array(&h.e, &[0u8; 64]),
        },
        proof: Bytes::new(&h.e),
    }
    .to_xdr(&h.e);
    wrapper.register(&alice, &1u32, &register_data);
}

#[test]
fn storage_keys_isolated_from_wrapper_keys() {
    // ComplianceStorageKey discriminants do not collide with
    // WrapperStorageKey ones: writing the compliance config does not
    // disturb the wrapper's stored token address.
    let h = setup();
    h.e.as_contract(&h.host, || {
        let before = crate::confidential::wrapper::storage::get_token(&h.e);
        set_compliance_config_no_auth(&h.e, &base_config());
        let after = crate::confidential::wrapper::storage::get_token(&h.e);
        assert_eq!(before, after);
        assert_eq!(after, h.sac_addr);
    });
}

// ################## HELPERS ##################

fn register_minimal_account(e: &Env, account: &Address) {
    // Bypass proof verification: the unregistered-deposit tests only need
    // `account_exists` to return true for selected addresses.
    use stellar_contract_utils::crypto::grumpkin::Grumpkin;

    use crate::confidential::wrapper::{ConfidentialAccount, WrapperStorageKey};
    let identity = Grumpkin::identity(e);
    let acc = ConfidentialAccount {
        spending_key: identity.clone(),
        viewing_public_key: identity.clone(),
        spendable_balance: identity.clone(),
        receiving_balance: identity,
        auditor_id: 0,
    };
    e.storage().persistent().set(&WrapperStorageKey::Account(account.clone()), &acc);
}
