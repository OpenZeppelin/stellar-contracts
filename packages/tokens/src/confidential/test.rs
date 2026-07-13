extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events, Ledger},
    token::StellarAssetClient,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env,
};

use crate::confidential::{
    auditor::{storage as auditor_storage, ConfidentialAuditor},
    storage as token_storage,
    verifier::{CircuitType, ConfidentialVerifier},
    ConfidentialAccount, ConfidentialToken, ConfidentialTokenClient, NoHooks, RegisterData,
    RegisterPayload, RevokeSpenderData, RevokeSpenderPayload, SetSpenderData, SetSpenderPayload,
    SpenderDelegation, SpenderTransferData, SpenderTransferPayload, TransferData, TransferPayload,
    WithdrawData, WithdrawPayload,
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
    let mut bytes = [byte; 32];
    // Zero the top byte so the 256-bit value is bounded by 2^248 - 1,
    // well below the BN254 scalar field modulus r ≈ 2^254. Lets the tests
    // keep using arbitrary sentinel bytes without tripping the
    // canonical-encoding check in `append_field` / `append_point`.
    bytes[0] = 0;
    BytesN::from_array(e, &bytes)
}

// ################## MOCK CONTRACTS ##################

#[contract]
struct BareContract;

#[contract]
struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn __constructor(e: &Env, token: Address, verifier: Address, auditor: Address) {
        token_storage::set_underlying_asset(e, &token);
        token_storage::set_verifier(e, &verifier);
        token_storage::set_auditor(e, &auditor);
        token_storage::set_address_as_field_element(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialToken for TokenContract {
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

/// Models the register verifier's proof-to-account binding for replay tests.
///
/// A real UltraHonk proof absorbs the `acct_f` public input into its
/// transcript, so it only verifies against the account it was produced for.
/// This mock stands in for that: the first proof it accepts fixes the bound
/// `acct_f` (the trailing 32-byte limb of the register blob), and every
/// later verification succeeds only when the contract-assembled `acct_f`
/// matches. A proof published by one registration therefore cannot be
/// replayed by a different registering address.
#[contract]
struct ReplayGuardVerifier;

#[contractimpl(contracttrait)]
impl ConfidentialVerifier for ReplayGuardVerifier {
    fn register_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn update_verification_key(_e: &Env, _ct: CircuitType, _vk: Bytes, _op: Address) {}

    fn verify_proof(e: &Env, _ct: CircuitType, pi: Bytes, _proof: Bytes) -> bool {
        let acct_f = pi.slice(pi.len() - 32..);
        let key = symbol_short!("bound");
        match e.storage().instance().get::<_, Bytes>(&key) {
            None => {
                e.storage().instance().set(&key, &acct_f);
                true
            }
            Some(bound) => bound == acct_f,
        }
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
    token: ConfidentialTokenClient<'a>,
    token_addr: Address,
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

    let token_addr =
        e.register(TokenContract, (sac_addr.clone(), verifier_addr.clone(), auditor_addr.clone()));
    let token = ConfidentialTokenClient::new(&e, &token_addr);

    Harness { e, token, token_addr, token_admin, sac: sac_client, sac_addr }
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

fn set_spender_data(e: &Env) -> Bytes {
    SetSpenderData {
        payload: SetSpenderPayload {
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

fn spender_transfer_data(e: &Env) -> Bytes {
    SpenderTransferData {
        payload: SpenderTransferPayload {
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

fn revoke_spender_data(e: &Env) -> Bytes {
    RevokeSpenderData {
        payload: RevokeSpenderPayload {
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

    h.token.register(&alice, &1u32, &register_data(&h.e));
    assert_eq!(h.e.events().all().events().len(), 1);

    let account: ConfidentialAccount = h.token.confidential_balance(&alice);
    assert_eq!(account.spending_key, fixture_point(&h.e));
    assert_eq!(account.viewing_public_key, fixture_point(&h.e));
    assert_eq!(account.auditor_id, 1u32);
    // Initial balances are identity (all-zero encoding).
    assert_eq!(account.spendable_balance.to_array(), [0u8; 64]);
    assert_eq!(account.receiving_balance.to_array(), [0u8; 64]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3506)")]
fn register_replayed_proof_bound_to_other_account_is_rejected() {
    let e = Env::default();
    let verifier_addr = e.register(ReplayGuardVerifier, ());
    let h = setup_with_verifier_addr(e, verifier_addr);

    let alice = Address::generate(&h.e);
    let mallory = Address::generate(&h.e);

    // Alice registers legitimately; her payload + proof are now public and the
    // verifier binds that proof to `address_to_field(alice)`.
    h.token.register(&alice, &1u32, &register_data(&h.e));

    // Mallory replays Alice's published material under his own address. The fix
    // makes the contract assemble the blob with `address_to_field(mallory)`, so
    // the proof — bound to Alice's acct_f — fails verification (InvalidProof,
    // #3506) instead of minting a duplicate-key account for Mallory.
    h.token.register(&mallory, &1u32, &register_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3500)")]
fn register_twice_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&alice, &1u32, &register_data(&h.e));
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
    h.token.register(&alice, &1u32, &wrong);
}

#[test]
#[should_panic(expected = "Error(Contract, #3506)")]
fn register_invalid_proof_panics() {
    let h = setup_with_failing_verifier();
    let alice = Address::generate(&h.e);
    h.token.register(&alice, &1u32, &register_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3514)")]
fn withdraw_non_canonical_scalar_panics() {
    // Sigma supplied as raw `[0xff; 32]` exceeds the BN254 scalar field
    // modulus r. The append helper must revert with `NonCanonicalEncoding`
    // before the bytes reach the verifier (which would otherwise silently
    // reduce them mod r and accept malleable encodings).
    let h = setup();
    let alice = Address::generate(&h.e);
    h.token.register(&alice, &1u32, &register_data(&h.e));

    let payload = WithdrawData {
        payload: WithdrawPayload {
            c_spend_new: fixture_point(&h.e),
            b_tilde: fixture_field(&h.e, 0x11),
            r_e: fixture_point(&h.e),
            sigma: BytesN::from_array(&h.e, &[0xff; 32]),
            b_aud_s: fixture_field(&h.e, 0x12),
        },
        proof: Bytes::new(&h.e),
    }
    .to_xdr(&h.e);

    h.token.withdraw(&alice, &Address::generate(&h.e), &0i128, &payload);
}

#[test]
#[should_panic(expected = "Error(Contract, #3514)")]
fn register_non_canonical_point_coord_panics() {
    // A point whose x-coordinate exceeds r. Both coordinates must be
    // canonical or `append_point` reverts before verification.
    let h = setup();
    let alice = Address::generate(&h.e);

    let mut non_canonical = [0u8; 64];
    non_canonical[..32].copy_from_slice(&[0xff; 32]);
    // y stays at zero; only x triggers the revert.

    let payload = RegisterData {
        payload: RegisterPayload {
            y: BytesN::from_array(&h.e, &non_canonical),
            pvk: fixture_point(&h.e),
        },
        proof: Bytes::new(&h.e),
    }
    .to_xdr(&h.e);

    h.token.register(&alice, &1u32, &payload);
}

#[test]
#[should_panic(expected = "Error(Contract, #3301)")]
fn register_unknown_auditor_panics() {
    // Routes through the auditor registry's `AuditorNotRegistered` (3301).
    let h = setup();
    let alice = Address::generate(&h.e);
    h.token.register(&alice, &999u32, &register_data(&h.e));
}

// ################## DEPOSIT ##################

#[test]
fn deposit_credits_receiving_balance() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);

    h.token.deposit(&depositor, &alice, &500i128);
    // 1 SAC transfer event + 1 Deposit event.
    assert_eq!(h.e.events().all().events().len(), 2);

    // Receiving balance must move off identity.
    let account = h.token.confidential_balance(&alice);
    assert_ne!(account.receiving_balance.to_array(), [0u8; 64]);
    // Contract now holds the deposit on the SEP-41 side.
    let token_client = soroban_sdk::token::TokenClient::new(&h.e, &h.sac_addr);
    assert_eq!(token_client.balance(&h.token_addr), 500);

    let _ = h.token_admin; // silence unused field for now
}

#[test]
#[should_panic(expected = "Error(Contract, #3502)")]
fn deposit_negative_amount_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.deposit(&depositor, &alice, &-1i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn deposit_to_unregistered_recipient_panics() {
    let h = setup();
    let depositor = Address::generate(&h.e);
    let unknown = Address::generate(&h.e);

    h.sac.mint(&depositor, &10i128);
    h.token.deposit(&depositor, &unknown, &5i128);
}

#[test]
fn deposit_zero_amount_is_ok() {
    // Deposit of 0 is allowed; receiving balance stays at identity (0·G = O).
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1i128);

    h.token.deposit(&depositor, &alice, &0i128);

    let account = h.token.confidential_balance(&alice);
    assert_eq!(account.receiving_balance.to_array(), [0u8; 64]);
}

// ################## MERGE ##################

#[test]
fn merge_folds_receiving_into_spendable() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);
    h.token.deposit(&depositor, &alice, &500i128);

    let pre = h.token.confidential_balance(&alice);
    h.token.merge(&alice);
    assert_eq!(h.e.events().all().events().len(), 1);

    let post = h.token.confidential_balance(&alice);

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
    h.token.merge(&stranger);
}

// ################## WITHDRAW ##################

#[test]
fn withdraw_transfers_tokens_and_updates_spendable() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);
    let beneficiary = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);
    h.token.deposit(&depositor, &alice, &1_000i128);
    h.token.merge(&alice);

    h.token.withdraw(&alice, &beneficiary, &300i128, &withdraw_data(&h.e));
    // 1 SAC transfer event + 1 Withdraw event.
    assert_eq!(h.e.events().all().events().len(), 2);

    let token_client = soroban_sdk::token::TokenClient::new(&h.e, &h.sac_addr);
    assert_eq!(token_client.balance(&beneficiary), 300);
    assert_eq!(token_client.balance(&h.token_addr), 700);

    let account = h.token.confidential_balance(&alice);
    // Spendable was overwritten by payload.c_spend_new.
    assert_eq!(account.spendable_balance, fixture_point(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3502)")]
fn withdraw_negative_amount_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let beneficiary = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.withdraw(&alice, &beneficiary, &-1i128, &withdraw_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn withdraw_from_unknown_account_panics() {
    let h = setup();
    let stranger = Address::generate(&h.e);
    let beneficiary = Address::generate(&h.e);

    h.token.withdraw(&stranger, &beneficiary, &1i128, &withdraw_data(&h.e));
}

// ################## CONFIDENTIAL TRANSFER ##################

#[test]
fn confidential_transfer_updates_both_sides() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let bob = Address::generate(&h.e);
    let depositor = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&bob, &1u32, &register_data(&h.e));
    h.sac.mint(&depositor, &1_000i128);
    h.token.deposit(&depositor, &alice, &1_000i128);
    h.token.merge(&alice);

    h.token.confidential_transfer(&alice, &bob, &transfer_data(&h.e));
    assert_eq!(h.e.events().all().events().len(), 1);

    // Sender's spendable balance was overwritten.
    let alice_acc = h.token.confidential_balance(&alice);
    assert_eq!(alice_acc.spendable_balance, fixture_point(&h.e));
    // Receiver's receiving balance accumulated.
    let bob_acc = h.token.confidential_balance(&bob);
    assert_ne!(bob_acc.receiving_balance.to_array(), [0u8; 64]);
}

// ################## SET / REVOKE SPENDER ##################

#[test]
fn set_spender_stores_delegation() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));

    h.token.set_spender(&alice, &spender, &1_000u32, &set_spender_data(&h.e));
    assert_eq!(h.e.events().all().events().len(), 1);

    let delegation = h.token.get_spender_delegation(&alice, &spender);
    assert_eq!(delegation.live_until_ledger, 1_000);
    assert!(h.token.is_spender(&alice, &spender));
}

#[test]
#[should_panic(expected = "Error(Contract, #3503)")]
fn set_spender_twice_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));
    h.token.set_spender(&alice, &spender, &1_000u32, &set_spender_data(&h.e));
    h.token.set_spender(&alice, &spender, &1_000u32, &set_spender_data(&h.e));
}

#[test]
fn revoke_spender_deletes_delegation() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));
    h.token.set_spender(&alice, &spender, &1_000u32, &set_spender_data(&h.e));

    h.token.revoke_spender(&alice, &spender, &revoke_spender_data(&h.e));
    assert_eq!(h.e.events().all().events().len(), 1);

    assert!(!h.token.is_spender(&alice, &spender));
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn revoke_unknown_spender_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.revoke_spender(&alice, &spender, &revoke_spender_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn get_spender_delegation_unknown_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    h.token.get_spender_delegation(&alice, &spender);
}

// ################## SPENDER TRANSFER ##################

#[test]
fn confidential_transfer_from_updates_delegation_and_recipient() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    let bob = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));
    h.token.register(&bob, &1u32, &register_data(&h.e));
    h.token.set_spender(&alice, &spender, &1_000u32, &set_spender_data(&h.e));

    h.token.confidential_transfer_from(&spender, &alice, &bob, &spender_transfer_data(&h.e));
    assert_eq!(h.e.events().all().events().len(), 1);

    // Delegation allowance commitment was rotated.
    let delegation = h.token.get_spender_delegation(&alice, &spender);
    assert_eq!(delegation.allowance_commitment, fixture_point(&h.e));
    // Bob's receiving balance accumulated.
    let bob_acc = h.token.confidential_balance(&bob);
    assert_ne!(bob_acc.receiving_balance.to_array(), [0u8; 64]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3505)")]
fn confidential_transfer_from_expired_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    let bob = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));
    h.token.register(&bob, &1u32, &register_data(&h.e));
    h.token.set_spender(&alice, &spender, &10u32, &set_spender_data(&h.e));

    // Advance past the delegation's expiration.
    h.e.ledger().set_sequence_number(100);

    h.token.confidential_transfer_from(&spender, &alice, &bob, &spender_transfer_data(&h.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3504)")]
fn confidential_transfer_from_no_delegation_panics() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);
    let bob = Address::generate(&h.e);

    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));
    h.token.register(&bob, &1u32, &register_data(&h.e));

    h.token.confidential_transfer_from(&spender, &alice, &bob, &spender_transfer_data(&h.e));
}

// ################## READ METHODS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3501)")]
fn confidential_balance_unknown_panics() {
    let h = setup();
    let unknown = Address::generate(&h.e);
    h.token.confidential_balance(&unknown);
}

#[test]
fn is_spender_returns_false_for_missing_and_expired() {
    let h = setup();
    let alice = Address::generate(&h.e);
    let spender = Address::generate(&h.e);

    // Missing.
    assert!(!h.token.is_spender(&alice, &spender));

    // Active.
    h.token.register(&alice, &1u32, &register_data(&h.e));
    h.token.register(&spender, &1u32, &register_data(&h.e));
    h.token.set_spender(&alice, &spender, &50u32, &set_spender_data(&h.e));
    assert!(h.token.is_spender(&alice, &spender));

    // Expired.
    h.e.ledger().set_sequence_number(100);
    assert!(!h.token.is_spender(&alice, &spender));

    // But the entry still exists (DESIGN §6.2).
    let _ = h.token.get_spender_delegation(&alice, &spender);
}

// ################## INSTANCE SETTERS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3513)")]
fn set_underlying_asset_twice_panics() {
    // `__constructor` already populated the entry; calling
    // `set_underlying_asset` again must trip `UnderlyingAssetAlreadySet`.
    let h = setup();
    let other_token = Address::generate(&h.e);
    h.e.as_contract(&h.token_addr, || {
        token_storage::set_underlying_asset(&h.e, &other_token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3512)")]
fn set_address_as_field_element_twice_panics() {
    // `__constructor` already populated the entry; calling
    // `set_address_as_field_element` again must trip `AddressAsFieldAlreadySet`.
    let h = setup();
    h.e.as_contract(&h.token_addr, || {
        token_storage::set_address_as_field_element(&h.e);
    });
}

#[test]
fn set_underlying_asset_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    let token = Address::generate(&e);
    e.as_contract(&bare, || {
        token_storage::set_underlying_asset(&e, &token);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn set_verifier_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    let verifier = Address::generate(&e);
    e.as_contract(&bare, || {
        token_storage::set_verifier(&e, &verifier);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn set_auditor_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    let auditor = Address::generate(&e);
    e.as_contract(&bare, || {
        token_storage::set_auditor(&e, &auditor);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn set_address_as_field_element_emits_event() {
    let e = Env::default();
    let bare = e.register(BareContract, ());
    e.as_contract(&bare, || {
        token_storage::set_address_as_field_element(&e);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

// Silence the otherwise-unused `SpenderDelegation` import.
#[test]
fn delegation_type_export_compiles() {
    fn _used<T>(_: T) {}
    let e = Env::default();
    _used::<SpenderDelegation>(SpenderDelegation {
        allowance_commitment: fixture_point(&e),
        encrypted_allowance: fixture_field(&e, 0),
        escrowed_dvk: fixture_point(&e),
        allowance_salt: fixture_field(&e, 0),
        live_until_ledger: 0,
    });
}
