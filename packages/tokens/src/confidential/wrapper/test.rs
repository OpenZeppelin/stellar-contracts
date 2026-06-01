extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env,
};
use stellar_event_assertion::EventAssertion;

use crate::confidential::{
    auditor::{storage as auditor_storage, ConfidentialAuditor},
    verifier::{CircuitType, ConfidentialVerifier},
    wrapper::{
        storage as wrapper_storage, ConfidentialAccount, ConfidentialTokenWrapper,
        ConfidentialTokenWrapperClient, NoHooks, OperatorDelegation, OperatorTransferData,
        OperatorTransferPayload, RegisterData, RegisterPayload, RevokeOperatorData,
        RevokeOperatorPayload, SetOperatorData, SetOperatorPayload, TransferData, TransferPayload,
        WithdrawData, WithdrawPayload,
    },
};

// ################## TEST FIXTURES ##################

/// Grumpkin generator `G = (1, Y)` with `Y =
/// 17631683881184975370165255887551781615748388533673675138860`. Used as a
/// canonical on-curve fixture for both auditor keys and account keys in
/// these tests.
const GRUMPKIN_G_BYTES: [u8; 64] = [
    // x = 1 (32-byte big-endian)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    // y (32-byte big-endian)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xcf, 0x13, 0x5e, 0x75, 0x06, 0xa4, 0x5d, 0x63,
    0x2d, 0x27, 0x0d, 0x45, 0xf1, 0x18, 0x12, 0x94, 0x83, 0x3f, 0xc4, 0x8d, 0x82, 0x3f, 0x27, 0x2c,
];

fn fixture_point(e: &Env) -> BytesN<64> {
    BytesN::from_array(e, &GRUMPKIN_G_BYTES)
}

fn fixture_field(e: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(e, &[byte; 32])
}

// ################## MOCK CONTRACTS ##################

#[contract]
struct BareContract;

#[contract]
struct WrapperContract;

#[contractimpl]
impl WrapperContract {
    pub fn __constructor(e: &Env, token: Address, verifier: Address, auditor: Address) {
        wrapper_storage::set_token_no_auth(e, &token);
        wrapper_storage::set_verifier_no_auth(e, &verifier);
        wrapper_storage::set_auditor_no_auth(e, &auditor);
        wrapper_storage::set_wrap_no_auth(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialTokenWrapper for WrapperContract {
    type Hooks = NoHooks;
}

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
struct AlwaysFailVerifier;

#[contractimpl(contracttrait)]
impl ConfidentialVerifier for AlwaysFailVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(_e: &Env, _ct: CircuitType, _pi: Bytes, _proof: Bytes) -> bool {
        false
    }
}

#[contract]
struct MockAuditor;

#[contractimpl(contracttrait)]
impl ConfidentialAuditor for MockAuditor {
    fn register_key(e: &Env, auditor_id: u32, point: BytesN<64>, _operator: Address) {
        auditor_storage::register_key(e, auditor_id, &point);
    }

    fn rotate_key(e: &Env, auditor_id: u32, new_point: BytesN<64>, _operator: Address) {
        auditor_storage::rotate_key(e, auditor_id, &new_point);
    }
}

// ################## SETUP HELPERS ##################

struct Harness<'a> {
    e: Env,
    wrapper: ConfidentialTokenWrapperClient<'a>,
    wrapper_addr: Address,
    token_admin: Address,
    sac: StellarAssetClient<'a>,
    sac_addr: Address,
}

fn setup<'a>() -> Harness<'a> {
    let e = Env::default();
    let verifier_addr = e.register(MockVerifier, ());
    setup_with_verifier_addr(e, verifier_addr)
}

fn setup_with_failing_verifier<'a>() -> Harness<'a> {
    let e = Env::default();
    let verifier_addr = e.register(AlwaysFailVerifier, ());
    setup_with_verifier_addr(e, verifier_addr)
}

fn setup_with_verifier_addr<'a>(e: Env, verifier_addr: Address) -> Harness<'a> {
    e.mock_all_auths();

    let token_admin = Address::generate(&e);
    let sac = e.register_stellar_asset_contract_v2(token_admin.clone());
    let sac_addr = sac.address();
    let sac_client = StellarAssetClient::new(&e, &sac_addr);

    let auditor_addr = e.register(MockAuditor, ());

    // Register an auditor key (id = 1) using the on-curve fixture so it
    // passes the validator inside `register_key`.
    let auditor_client =
        crate::confidential::auditor::ConfidentialAuditorClient::new(&e, &auditor_addr);
    auditor_client.register_key(&1u32, &fixture_point(&e), &Address::generate(&e));

    let wrapper_addr = e
        .register(WrapperContract, (sac_addr.clone(), verifier_addr.clone(), auditor_addr.clone()));
    let wrapper = ConfidentialTokenWrapperClient::new(&e, &wrapper_addr);

    Harness { e, wrapper, wrapper_addr, token_admin, sac: sac_client, sac_addr }
}

fn register_data(e: &Env) -> Bytes {
    RegisterData {
        payload: RegisterPayload { y: fixture_point(e), pvk: fixture_point(e) },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn withdraw_data(e: &Env) -> Bytes {
    WithdrawData {
        payload: WithdrawPayload {
            c_spend_new: fixture_point(e),
            b_tilde: fixture_field(e, 0xaa),
            r_e: fixture_point(e),
            sigma: fixture_field(e, 0xbb),
            b_aud_s: fixture_field(e, 0xcc),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn transfer_data(e: &Env) -> Bytes {
    TransferData {
        payload: TransferPayload {
            c_spend_new: fixture_point(e),
            c_tx: fixture_point(e),
            r_e: fixture_point(e),
            v_tilde: fixture_field(e, 0x11),
            b_tilde: fixture_field(e, 0x12),
            sigma: fixture_field(e, 0x13),
            v_aud_r: fixture_field(e, 0x14),
            r_aud_r: fixture_field(e, 0x15),
            v_aud_s: fixture_field(e, 0x16),
            b_aud_s: fixture_field(e, 0x17),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn set_operator_data(e: &Env) -> Bytes {
    SetOperatorData {
        payload: SetOperatorPayload {
            c_spend_new: fixture_point(e),
            c_a: fixture_point(e),
            escrowed_dvk: fixture_point(e),
            b_tilde: fixture_field(e, 0x21),
            a_tilde: fixture_field(e, 0x22),
            r_e: fixture_point(e),
            sigma: fixture_field(e, 0x23),
            sigma_a: fixture_field(e, 0x24),
            v_aud_s: fixture_field(e, 0x25),
            b_aud_s: fixture_field(e, 0x26),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn operator_transfer_data(e: &Env) -> Bytes {
    OperatorTransferData {
        payload: OperatorTransferPayload {
            c_a_new: fixture_point(e),
            c_tx: fixture_point(e),
            r_e: fixture_point(e),
            v_tilde: fixture_field(e, 0x31),
            a_tilde_new: fixture_field(e, 0x32),
            sigma_a_new: fixture_field(e, 0x33),
            v_aud_r: fixture_field(e, 0x34),
            r_aud_r: fixture_field(e, 0x35),
            v_aud_s: fixture_field(e, 0x36),
            a_aud_s: fixture_field(e, 0x37),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn revoke_operator_data(e: &Env) -> Bytes {
    RevokeOperatorData {
        payload: RevokeOperatorPayload {
            c_spend_new: fixture_point(e),
            b_tilde: fixture_field(e, 0x41),
            r_e: fixture_point(e),
            sigma: fixture_field(e, 0x42),
            v_aud_s: fixture_field(e, 0x43),
            b_aud_s: fixture_field(e, 0x44),
        },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

// ################## REGISTER ##################

#[test]
fn register_stores_account_with_identity_balances() {
    let h = setup();
    let alice = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(1);

    let account: ConfidentialAccount = h.wrapper.confidential_balance(&alice);
    assert_eq!(account.spending_key, fixture_point(&h.e));
    assert_eq!(account.viewing_public_key, fixture_point(&h.e));
    assert_eq!(account.auditor_id, 1u32);
    // Initial balances are identity (all-zero encoding).
    assert_eq!(account.spendable_balance.to_array(), [0u8; 64]);
    assert_eq!(account.receiving_balance.to_array(), [0u8; 64]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3500)")]
fn register_twice_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3507)")]
fn register_wrong_type_payload_panics() {
    // Valid XDR encoding a different type (`u32`) decodes into a Val that
    // doesn't satisfy `RegisterPayload`, exercising the `InvalidDataPayload`
    // branch of `decode_payload`.
    let h = setup();
    let alice = Address::generate(&h.e);
    let wrong = 42u32.to_xdr(&h.e);
    h.wrapper.register(&alice, &1u32, &wrong);
}

#[test]
#[should_panic(expected = "Error(Contract, #3506)")]
fn register_invalid_proof_panics() {
    let h = setup_with_failing_verifier();
    let alice = Address::generate(&h.e);
    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn register_unknown_auditor_panics() {
    // Routes through the auditor registry's `AuditorNotRegistered` (3301).
    let h = setup();
    let alice = Address::generate(&h.e);
    h.wrapper.register(&alice, &999u32, &register_data(&h.e));
}

// ################## DEPOSIT ##################

#[test]
fn deposit_credits_receiving_balance() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);

    h.wrapper.deposit(&depositor, &alice, &500i128);
    // 1 SAC transfer event + 1 Deposit event.
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(2);

    // Receiving balance must move off identity.
    let account = h.wrapper.confidential_balance(&alice);
    assert_ne!(account.receiving_balance.to_array(), [0u8; 64]);
    // Wrapper now holds the deposit on the SEP-41 side.
    let token_client = soroban_sdk::token::TokenClient::new(&h.e, &h.sac_addr);
    assert_eq!(token_client.balance(&h.wrapper_addr), 500);

    let _ = h.token_admin; // silence unused field for now
}

#[test]
#[should_panic(expected = "Error(Contract, #3502)")]
fn deposit_negative_amount_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.deposit(&depositor, &alice, &-1i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn deposit_to_unregistered_recipient_panics() {
    let h = setup();
    let depositor = Address::generate(&h.e);
    let unknown = Address::generate(&h.e);

    h.sac.mint(&depositor, &10i128);
    h.wrapper.deposit(&depositor, &unknown, &5i128);
}

#[test]
fn deposit_zero_amount_is_ok() {
    // Deposit of 0 is allowed; receiving balance stays at identity (0·G = O).
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1i128);

    h.wrapper.deposit(&depositor, &alice, &0i128);

    let account = h.wrapper.confidential_balance(&alice);
    assert_eq!(account.receiving_balance.to_array(), [0u8; 64]);
}

// ################## MERGE ##################

#[test]
fn merge_folds_receiving_into_spendable() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);
    h.wrapper.deposit(&depositor, &alice, &500i128);

    let pre = h.wrapper.confidential_balance(&alice);
    h.wrapper.merge(&alice);
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(1);

    let post = h.wrapper.confidential_balance(&alice);

    // Spendable now equals the prior receiving (since prior spendable was
    // identity).
    assert_eq!(post.spendable_balance, pre.receiving_balance);
    assert_eq!(post.receiving_balance.to_array(), [0u8; 64]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn merge_unknown_account_panics() {
    let h = setup();
    let stranger = Address::generate(&h.e);
    h.wrapper.merge(&stranger);
}

// ################## WITHDRAW ##################

#[test]
fn withdraw_transfers_tokens_and_updates_spendable() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);
    let beneficiary = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);
    h.wrapper.deposit(&depositor, &alice, &1_000i128);
    h.wrapper.merge(&alice);

    h.wrapper.withdraw(&alice, &beneficiary, &300i128, &withdraw_data(&h.e));
    // 1 SAC transfer event + 1 Withdraw event.
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(2);

    let token_client = soroban_sdk::token::TokenClient::new(&h.e, &h.sac_addr);
    assert_eq!(token_client.balance(&beneficiary), 300);
    assert_eq!(token_client.balance(&h.wrapper_addr), 700);

    let account = h.wrapper.confidential_balance(&alice);
    // Spendable was overwritten by payload.c_spend_new.
    assert_eq!(account.spendable_balance, fixture_point(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3502)")]
fn withdraw_negative_amount_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let beneficiary = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.withdraw(&alice, &beneficiary, &-1i128, &withdraw_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn withdraw_from_unknown_account_panics() {
    let h = setup();
    let stranger = Address::generate(&h.e);
    let beneficiary = Address::generate(&h.e);

    h.wrapper.withdraw(&stranger, &beneficiary, &1i128, &withdraw_data(&h.e));
}

// ################## CONFIDENTIAL TRANSFER ##################

#[test]
fn confidential_transfer_updates_both_sides() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&bob, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);
    h.wrapper.deposit(&depositor, &alice, &1_000i128);
    h.wrapper.merge(&alice);

    h.wrapper.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(1);

    // Sender's spendable balance was overwritten.
    let alice_acc = h.wrapper.confidential_balance(&alice);
    assert_eq!(alice_acc.spendable_balance, fixture_point(&h.e));
    // Receiver's receiving balance accumulated.
    let bob_acc = h.wrapper.confidential_balance(&bob);
    assert_ne!(bob_acc.receiving_balance.to_array(), [0u8; 64]);
}

// ################## SET / REVOKE OPERATOR ##################

#[test]
fn set_operator_stores_delegation() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));

    h.wrapper.set_operator(&alice, &operator, &1_000u32, &set_operator_data(&h.e));
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(1);

    let delegation = h.wrapper.get_operator(&alice, &operator);
    assert_eq!(delegation.expiration_ledger, 1_000);
    assert!(h.wrapper.is_operator(&alice, &operator));
}

#[test]
#[should_panic(expected = "Error(Contract, #3503)")]
fn set_operator_twice_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));
    h.wrapper.set_operator(&alice, &operator, &1_000u32, &set_operator_data(&h.e));
    h.wrapper.set_operator(&alice, &operator, &1_000u32, &set_operator_data(&h.e));
}

#[test]
fn revoke_operator_deletes_delegation() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));
    h.wrapper.set_operator(&alice, &operator, &1_000u32, &set_operator_data(&h.e));

    h.wrapper.revoke_operator(&alice, &operator, &revoke_operator_data(&h.e));
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(1);

    assert!(!h.wrapper.is_operator(&alice, &operator));
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn revoke_unknown_operator_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);
    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.revoke_operator(&alice, &operator, &revoke_operator_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn get_operator_unknown_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);
    h.wrapper.get_operator(&alice, &operator);
}

// ################## OPERATOR TRANSFER ##################

#[test]
fn confidential_transfer_from_updates_delegation_and_recipient() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);
    let bob = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));
    h.wrapper.register(&bob, &1u32, &register_data(&h.e));
    h.wrapper.set_operator(&alice, &operator, &1_000u32, &set_operator_data(&h.e));

    h.wrapper.confidential_transfer_from(&operator, &alice, &bob, &operator_transfer_data(&h.e));
    EventAssertion::new(&h.e, h.wrapper_addr.clone()).assert_event_count(1);

    // Delegation allowance commitment was rotated.
    let delegation = h.wrapper.get_operator(&alice, &operator);
    assert_eq!(delegation.allowance_commitment, fixture_point(&h.e));
    // Bob's receiving balance accumulated.
    let bob_acc = h.wrapper.confidential_balance(&bob);
    assert_ne!(bob_acc.receiving_balance.to_array(), [0u8; 64]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3505)")]
fn confidential_transfer_from_expired_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);
    let bob = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));
    h.wrapper.register(&bob, &1u32, &register_data(&h.e));
    h.wrapper.set_operator(&alice, &operator, &10u32, &set_operator_data(&h.e));

    // Advance past the delegation's expiration.
    h.e.ledger().set_sequence_number(100);

    h.wrapper.confidential_transfer_from(&operator, &alice, &bob, &operator_transfer_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn confidential_transfer_from_no_delegation_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);
    let bob = Address::generate(&h.e);

    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));
    h.wrapper.register(&bob, &1u32, &register_data(&h.e));

    h.wrapper.confidential_transfer_from(&operator, &alice, &bob, &operator_transfer_data(&h.e));
}

// ################## READ METHODS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn confidential_balance_unknown_panics() {
    let h = setup();
    let unknown = Address::generate(&h.e);
    h.wrapper.confidential_balance(&unknown);
}

#[test]
fn is_operator_returns_false_for_missing_and_expired() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let operator = Address::generate(&h.e);

    // Missing.
    assert!(!h.wrapper.is_operator(&alice, &operator));

    // Active.
    h.wrapper.register(&alice, &1u32, &register_data(&h.e));
    h.wrapper.register(&operator, &1u32, &register_data(&h.e));
    h.wrapper.set_operator(&alice, &operator, &50u32, &set_operator_data(&h.e));
    assert!(h.wrapper.is_operator(&alice, &operator));

    // Expired.
    h.e.ledger().set_sequence_number(100);
    assert!(!h.wrapper.is_operator(&alice, &operator));

    // But the entry still exists (DESIGN §6.2).
    let _ = h.wrapper.get_operator(&alice, &operator);
}

// ################## INSTANCE SETTERS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3513)")]
fn set_token_twice_panics() {
    // `__constructor` already populated the entry; calling `set_token` again
    // must trip `TokenAlreadySet`.
    let h = setup();
    let other_token = Address::generate(&h.e);
    h.e.as_contract(&h.wrapper_addr, || {
        wrapper_storage::set_token_no_auth(&h.e, &other_token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3512)")]
fn set_wrap_twice_panics() {
    // `__constructor` already populated the entry; calling `set_wrap` again
    // must trip `WrapAlreadySet`.
    let h = setup();
    h.e.as_contract(&h.wrapper_addr, || {
        wrapper_storage::set_wrap_no_auth(&h.e);
    });
}

#[test]
fn set_token_no_auth_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    let token = Address::generate(&e);
    e.as_contract(&bare, || {
        wrapper_storage::set_token_no_auth(&e, &token);
        EventAssertion::new(&e, bare.clone()).assert_event_count(1);
    });
}

#[test]
fn set_verifier_no_auth_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    let verifier = Address::generate(&e);
    e.as_contract(&bare, || {
        wrapper_storage::set_verifier_no_auth(&e, &verifier);
        EventAssertion::new(&e, bare.clone()).assert_event_count(1);
    });
}

#[test]
fn set_auditor_no_auth_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    let auditor = Address::generate(&e);
    e.as_contract(&bare, || {
        wrapper_storage::set_auditor_no_auth(&e, &auditor);
        EventAssertion::new(&e, bare.clone()).assert_event_count(1);
    });
}

#[test]
fn set_wrap_no_auth_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    e.as_contract(&bare, || {
        wrapper_storage::set_wrap_no_auth(&e);
        EventAssertion::new(&e, bare.clone()).assert_event_count(1);
    });
}

// Silence the otherwise-unused `OperatorDelegation` import.
#[test]
fn delegation_type_export_compiles() {
    fn _used<T>(_: T) {}
    let e = Env::default();
    _used::<OperatorDelegation>(OperatorDelegation {
        allowance_commitment: fixture_point(&e),
        encrypted_allowance: fixture_field(&e, 0),
        escrowed_dvk: fixture_point(&e),
        allowance_salt: fixture_field(&e, 0),
        expiration_ledger: 0,
    });
}
