extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Events, MockAuth, MockAuthInvoke},
    token::StellarAssetClient,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, Error, Event, IntoVal, Symbol,
};
use stellar_access::access_control::AccessControlError;
use stellar_confidential::{
    auditor::{storage as auditor_storage, ConfidentialAuditor, ConfidentialAuditorClient},
    compliance::{
        storage::transfer_approval_args, ClawbackData, ComplianceConfig, ComplianceError,
        ForcedTransfer, Policy, RwaConfig,
    },
    verifier::{CircuitType, ConfidentialVerifier},
    RegisterData, RegisterPayload, TransferData, TransferPayload,
};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::contract::{RwaTokenContract, RwaTokenContractClient};

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

/// Stands in for the identity policy of a regulated deployment: an
/// allowlist of verified investors.
#[contract]
struct MockAllowlist;

#[contractimpl]
impl MockAllowlist {
    pub fn allow(e: &Env, account: Address) {
        e.storage().persistent().set(&MockAllowlistKey::Allowed(account), &());
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
    token: RwaTokenContractClient<'a>,
    allowlist: MockAllowlistClient<'a>,
    admin: Address,
    freezer: Address,
    config_manager: Address,
    clawback: Address,
    forced_transfer: Address,
    treasury: Address,
}

fn setup<'a>(approver: Option<Address>) -> Harness<'a> {
    let e = Env::default();
    e.mock_all_auths();
    deploy(e, approver)
}

fn deploy<'a>(e: Env, approver: Option<Address>) -> Harness<'a> {
    let sac = e.register_stellar_asset_contract_v2(Address::generate(&e));
    let sac = StellarAssetClient::new(&e, &sac.address());

    let verifier = e.register(MockVerifier, ());
    let auditor = e.register(MockAuditor, ());
    ConfidentialAuditorClient::new(&e, &auditor).register_key(
        &AUDITOR_ID,
        &Grumpkin::generator(&e),
        &Address::generate(&e),
    );
    let allowlist = MockAllowlistClient::new(&e, &e.register(MockAllowlist, ()));

    let admin = Address::generate(&e);
    let address = e.register(
        RwaTokenContract,
        (
            admin.clone(),
            sac.address.clone(),
            verifier,
            auditor,
            ComplianceConfig { policy: Some(allowlist.address.clone()), sac_passthrough: false },
            AUDITOR_ID,
            approver,
        ),
    );
    let token = RwaTokenContractClient::new(&e, &address);

    let freezer = Address::generate(&e);
    let config_manager = Address::generate(&e);
    let clawback = Address::generate(&e);
    let forced_transfer = Address::generate(&e);
    token.grant_role(&freezer, &Symbol::new(&e, "freezer"), &admin);
    token.grant_role(&config_manager, &Symbol::new(&e, "config_manager"), &admin);
    token.grant_role(&clawback, &Symbol::new(&e, "clawback"), &admin);
    token.grant_role(&forced_transfer, &Symbol::new(&e, "forced_transfer"), &admin);

    let treasury = Address::generate(&e);
    Harness {
        e,
        sac,
        token,
        allowlist,
        admin,
        freezer,
        config_manager,
        clawback,
        forced_transfer,
        treasury,
    }
}

impl Harness<'_> {
    /// Onboards `account`: allowlists it and registers its confidential
    /// account under the designated auditor.
    fn onboard(&self, account: &Address) {
        self.allowlist.allow(account);
        self.token.register(account, &AUDITOR_ID, &register_data(&self.e));
    }

    fn investor(&self) -> Address {
        let a = Address::generate(&self.e);
        self.onboard(&a);
        a
    }

    /// The issuance flow: mint to the treasury, deposit, merge.
    fn issue_to_treasury(&self, amount: i128) {
        self.onboard(&self.treasury);
        self.sac.mint(&self.treasury, &amount);
        self.token.deposit(&self.treasury, &self.treasury, &amount);
        self.token.merge(&self.treasury);
    }
}

fn field(e: &Env, byte: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[31] = byte;
    BytesN::from_array(e, &b)
}

fn register_data(e: &Env) -> Bytes {
    RegisterData {
        payload: RegisterPayload { y: Grumpkin::generator(e), pvk: Grumpkin::generator(e) },
        proof: Bytes::new(e),
    }
    .to_xdr(e)
}

fn transfer_payload(e: &Env) -> TransferPayload {
    TransferPayload {
        c_spend_new: Grumpkin::generator(e),
        c_transfer: Grumpkin::generator(e),
        r_e_point: Grumpkin::generator(e),
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

fn transfer_data(e: &Env) -> Bytes {
    TransferData { payload: transfer_payload(e), proof: Bytes::new(e) }.to_xdr(e)
}

fn proof_data(e: &Env) -> Bytes {
    ClawbackData { proof: Bytes::new(e) }.to_xdr(e)
}

fn contract_error(code: u32) -> Error {
    Error::from_contract_error(code)
}

// ################## CONSTRUCTION ##################

#[test]
fn constructor_writes_both_configurations() {
    let h = setup(None);
    assert_eq!(h.token.rwa_config(), Some(RwaConfig { auditor_id: AUDITOR_ID, approver: None }));
    assert_eq!(h.token.approval_epoch(), 1);
    assert!(h.token.compliance_config().is_some());
}

#[test]
#[should_panic(expected = "Error(Contract, #3615)")]
fn constructor_rejects_missing_eligibility_gate() {
    let e = Env::default();
    e.mock_all_auths();
    let sac = e.register_stellar_asset_contract_v2(Address::generate(&e));
    let auditor = e.register(MockAuditor, ());
    ConfidentialAuditorClient::new(&e, &auditor).register_key(
        &AUDITOR_ID,
        &Grumpkin::generator(&e),
        &Address::generate(&e),
    );
    e.register(
        RwaTokenContract,
        (
            Address::generate(&e),
            sac.address(),
            e.register(MockVerifier, ()),
            auditor,
            ComplianceConfig { policy: None, sac_passthrough: false },
            AUDITOR_ID,
            None::<Address>,
        ),
    );
}

// ################## ROLES ##################

#[test]
fn rwa_entry_points_require_their_roles() {
    let h = setup(None);
    let outsider = Address::generate(&h.e);
    let unauthorized = Error::from_contract_error(AccessControlError::Unauthorized as u32);

    assert_eq!(
        h.token.try_set_rwa_config(&AUDITOR_ID, &None, &outsider).unwrap_err().unwrap(),
        unauthorized
    );
    assert_eq!(h.token.try_bump_approval_epoch(&outsider).unwrap_err().unwrap(), unauthorized);
    assert_eq!(
        h.token
            .try_forced_transfer(&outsider, &outsider, &1, &proof_data(&h.e), &outsider)
            .unwrap_err()
            .unwrap(),
        unauthorized
    );
    // A role does not extend to another duty.
    assert_eq!(h.token.try_bump_approval_epoch(&h.freezer).unwrap_err().unwrap(), unauthorized);
    assert_eq!(
        h.token
            .try_forced_transfer(&outsider, &outsider, &1, &proof_data(&h.e), &h.config_manager)
            .unwrap_err()
            .unwrap(),
        unauthorized
    );
}

#[test]
fn compliance_entry_points_require_their_roles() {
    let h = setup(None);
    let outsider = Address::generate(&h.e);
    let account = h.investor();
    let unauthorized = Error::from_contract_error(AccessControlError::Unauthorized as u32);
    let config = h.token.compliance_config().unwrap();

    assert_eq!(h.token.try_freeze(&account, &outsider).unwrap_err().unwrap(), unauthorized);
    assert_eq!(h.token.try_unfreeze(&account, &outsider).unwrap_err().unwrap(), unauthorized);
    assert_eq!(
        h.token.try_set_compliance_config(&config, &outsider).unwrap_err().unwrap(),
        unauthorized
    );
    assert_eq!(
        h.token
            .try_clawback(&account, &1, &None, &proof_data(&h.e), &outsider)
            .unwrap_err()
            .unwrap(),
        unauthorized
    );
    assert_eq!(
        h.token.try_force_revoke_spender(&account, &outsider, &outsider).unwrap_err().unwrap(),
        unauthorized
    );
    // The forced-transfer role does not carry the clawback duty, nor the
    // reverse.
    assert_eq!(
        h.token
            .try_clawback(&account, &1, &None, &proof_data(&h.e), &h.forced_transfer)
            .unwrap_err()
            .unwrap(),
        unauthorized
    );
    assert_eq!(
        h.token
            .try_forced_transfer(&account, &account, &1, &proof_data(&h.e), &h.clawback)
            .unwrap_err()
            .unwrap(),
        unauthorized
    );
}

#[test]
fn freeze_clawback_and_unfreeze_by_their_roles() {
    let h = setup(None);
    let account = h.investor();
    h.sac.mint(&account, &100);
    h.token.deposit(&account, &account, &100);

    h.token.freeze(&account, &h.freezer);
    assert!(!h.token.can_receive(&account));
    h.token.clawback(&account, &40, &None, &proof_data(&h.e), &h.clawback);
    assert_eq!(
        h.token.confidential_balance(&account).spendable_commitment,
        Grumpkin::mul(&h.e, &Grumpkin::generator(&h.e), 60)
    );
    h.token.unfreeze(&account, &h.freezer);
    assert!(h.token.can_receive(&account));
}

#[test]
fn config_manager_changes_the_policy_and_the_epoch_advances() {
    let h = setup(None);
    let config =
        ComplianceConfig { policy: Some(h.allowlist.address.clone()), sac_passthrough: true };
    h.token.set_compliance_config(&config, &h.config_manager);
    assert_eq!(h.token.compliance_config(), Some(config));
    assert_eq!(h.token.approval_epoch(), 2);

    // The eligibility gate cannot be removed.
    assert_eq!(
        h.token
            .try_set_compliance_config(
                &ComplianceConfig { policy: None, sac_passthrough: false },
                &h.config_manager
            )
            .unwrap_err()
            .unwrap(),
        contract_error(ComplianceError::EligibilityRequired as u32)
    );
}

#[test]
fn config_manager_rotates_the_approver() {
    let h = setup(None);
    let approver = Address::generate(&h.e);
    h.token.set_rwa_config(&AUDITOR_ID, &Some(approver.clone()), &h.config_manager);
    assert_eq!(
        h.token.rwa_config(),
        Some(RwaConfig { auditor_id: AUDITOR_ID, approver: Some(approver) })
    );
    assert_eq!(h.token.approval_epoch(), 2);

    h.token.bump_approval_epoch(&h.config_manager);
    assert_eq!(h.token.approval_epoch(), 3);
}

#[test]
fn designated_auditor_cannot_be_changed() {
    let h = setup(None);
    assert_eq!(
        h.token.try_set_rwa_config(&7, &None, &h.config_manager).unwrap_err().unwrap(),
        contract_error(ComplianceError::AuditorAlreadySet as u32)
    );
}

// ################## TIER 0: MONITOR AND REMEDIATE ##################

#[test]
fn issuance_and_transfer_between_verified_investors() {
    let h = setup(None);
    h.issue_to_treasury(1_000);
    let alice = h.investor();

    assert!(h.token.can_send(&h.treasury));
    assert!(h.token.can_receive(&alice));
    h.token.confidential_transfer(&h.treasury, &alice, &transfer_data(&h.e));

    assert_eq!(
        h.token.confidential_balance(&alice).receiving_commitment,
        Grumpkin::generator(&h.e)
    );
}

#[test]
fn unverified_address_cannot_register_or_receive() {
    let h = setup(None);
    h.issue_to_treasury(1_000);
    let stranger = Address::generate(&h.e);

    assert_eq!(
        h.token.try_register(&stranger, &AUDITOR_ID, &register_data(&h.e)).unwrap_err().unwrap(),
        contract_error(ComplianceError::NotAuthorizedByPolicy as u32)
    );
    assert!(!h.token.can_receive(&stranger));
    assert_eq!(
        h.token
            .try_confidential_transfer(&h.treasury, &stranger, &transfer_data(&h.e))
            .unwrap_err()
            .unwrap(),
        contract_error(ComplianceError::NotAuthorizedByPolicy as u32)
    );
}

#[test]
fn registration_under_another_auditor_is_rejected() {
    let h = setup(None);
    let alice = Address::generate(&h.e);
    h.allowlist.allow(&alice);
    assert_eq!(
        h.token.try_register(&alice, &1, &register_data(&h.e)).unwrap_err().unwrap(),
        contract_error(ComplianceError::AuditorNotAllowed as u32)
    );
}

#[test]
fn lost_account_is_recovered_by_forced_transfer() {
    let h = setup(None);
    let lost = h.investor();
    h.sac.mint(&lost, &300);
    h.token.deposit(&lost, &lost, &300);
    let fresh = h.investor();

    h.token.freeze(&lost, &h.freezer);
    h.token.forced_transfer(&lost, &fresh, &300, &proof_data(&h.e), &h.forced_transfer);

    let events = h.e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &ForcedTransfer { from: lost.clone(), to: fresh.clone(), amount: 300, nonce: 0 }
            .to_xdr(&h.e, &h.token.address),
    );
    assert_eq!(
        h.token.confidential_balance(&fresh).receiving_commitment,
        Grumpkin::mul(&h.e, &Grumpkin::generator(&h.e), 300)
    );
    assert_eq!(h.token.confidential_balance(&lost).spendable_commitment, Grumpkin::identity(&h.e));
    // The pool still backs every claim.
    assert_eq!(h.sac.balance(&h.token.address), 300);
    assert_eq!(h.token.clawback_nonce(&lost), 1);
}

#[test]
fn forced_transfer_requires_a_frozen_source() {
    let h = setup(None);
    let a = h.investor();
    let b = h.investor();
    assert_eq!(
        h.token
            .try_forced_transfer(&a, &b, &1, &proof_data(&h.e), &h.forced_transfer)
            .unwrap_err()
            .unwrap(),
        contract_error(ComplianceError::AccountNotFrozen as u32)
    );
}

// ################## TIER 1: APPROVAL GATE ##################

/// Submits `confidential_transfer(treasury, to, data)` with the treasury's
/// authorization and, when `approved`, the approver's authorization over the
/// approval arguments.
fn submit_transfer(h: &Harness, approver: &Address, to: &Address, approved: bool) {
    let e = &h.e;
    let data = transfer_data(e);
    let sender = MockAuthInvoke {
        contract: &h.token.address,
        fn_name: "confidential_transfer",
        args: (h.treasury.clone(), to.clone(), data.clone()).into_val(e),
        sub_invokes: &[],
    };
    let approval_args = e.as_contract(&h.token.address, || {
        transfer_approval_args(e, &h.treasury, to, &transfer_payload(e))
    });
    let approval = MockAuthInvoke {
        contract: &h.token.address,
        fn_name: "confidential_transfer",
        args: approval_args,
        sub_invokes: &[],
    };
    if approved {
        h.token
            .mock_auths(&[
                MockAuth { address: &h.treasury, invoke: &sender },
                MockAuth { address: approver, invoke: &approval },
            ])
            .confidential_transfer(&h.treasury, to, &data);
    } else {
        h.token
            .mock_auths(&[MockAuth { address: &h.treasury, invoke: &sender }])
            .confidential_transfer(&h.treasury, to, &data);
    }
}

fn approved_deployment<'a>() -> (Harness<'a>, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let approver = Address::generate(&e);
    let h = deploy(e, Some(approver.clone()));
    h.issue_to_treasury(1_000);
    let alice = h.investor();
    (h, approver, alice)
}

#[test]
fn transfer_settles_with_the_approver_authorization() {
    let (h, approver, alice) = approved_deployment();
    submit_transfer(&h, &approver, &alice, true);
    assert_eq!(
        h.token.confidential_balance(&alice).receiving_commitment,
        Grumpkin::generator(&h.e)
    );
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn transfer_fails_without_the_approver_authorization() {
    let (h, approver, alice) = approved_deployment();
    submit_transfer(&h, &approver, &alice, false);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn approval_is_void_after_an_epoch_bump() {
    let (h, approver, alice) = approved_deployment();
    let e = &h.e;
    // The approval is built for the current epoch...
    let stale_args = e.as_contract(&h.token.address, || {
        transfer_approval_args(e, &h.treasury, &alice, &transfer_payload(e))
    });
    // ...and the issuer voids outstanding approvals before it is used.
    h.token.bump_approval_epoch(&h.config_manager);

    let data = transfer_data(e);
    let sender = MockAuthInvoke {
        contract: &h.token.address,
        fn_name: "confidential_transfer",
        args: (h.treasury.clone(), alice.clone(), data.clone()).into_val(e),
        sub_invokes: &[],
    };
    let approval = MockAuthInvoke {
        contract: &h.token.address,
        fn_name: "confidential_transfer",
        args: stale_args,
        sub_invokes: &[],
    };
    h.token
        .mock_auths(&[
            MockAuth { address: &h.treasury, invoke: &sender },
            MockAuth { address: &approver, invoke: &approval },
        ])
        .confidential_transfer(&h.treasury, &alice, &data);
}

#[test]
fn admin_is_not_the_approver() {
    // Holding every role does not satisfy the approval gate.
    let (h, _approver, alice) = approved_deployment();
    let e = &h.e;
    let data = transfer_data(e);
    let sender = MockAuthInvoke {
        contract: &h.token.address,
        fn_name: "confidential_transfer",
        args: (h.treasury.clone(), alice.clone(), data.clone()).into_val(e),
        sub_invokes: &[],
    };
    let approval_args = e.as_contract(&h.token.address, || {
        transfer_approval_args(e, &h.treasury, &alice, &transfer_payload(e))
    });
    let forged = MockAuthInvoke {
        contract: &h.token.address,
        fn_name: "confidential_transfer",
        args: approval_args,
        sub_invokes: &[],
    };
    let res = h
        .token
        .mock_auths(&[
            MockAuth { address: &h.treasury, invoke: &sender },
            MockAuth { address: &h.admin, invoke: &forged },
        ])
        .try_confidential_transfer(&h.treasury, &alice, &data);
    assert!(res.is_err());
}
