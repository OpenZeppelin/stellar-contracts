extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    contract, contractimpl, contracttype, panic_with_error, symbol_short,
    testutils::{
        Address as _, AuthorizedFunction, AuthorizedInvocation, Events, Ledger, MockAuth,
        MockAuthInvoke,
    },
    vec,
    xdr::{ScErrorCode, ScErrorType},
    Address, Bytes, Env, Error, Event, IntoVal, Map, String, Symbol, TryFromVal, Val, Vec,
};
use stellar_accounts::{
    policies::social_recovery::{
        hash_signers, RecoveryAttempt, RecoveryAuthorized, SocialRecoveryAccountParams,
        RECOVERED_RULE_NAME,
    },
    smart_account::{ContextRule, ContextRuleType, Signer},
};

use crate::contract::{SocialRecoveryPolicyContract, SocialRecoveryPolicyContractClient};

/// The error a [`MockVerifier`] in [`VerifierMode::Fail`] panics with.
const VERIFIER_FAILURE: u32 = 99;

#[allow(clippy::large_enum_variant)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum VerifierMode {
    /// Returns each key unchanged.
    Canonical,
    /// Panics with [`VERIFIER_FAILURE`].
    Fail,
    /// Calls `approve(guardian, 1, [Delegated(guardian)], RULE_ID,
    /// smart_account)` on the policy.
    CallPolicy(Address, Address, Address),
}

#[contract]
struct MockVerifier;

#[contractimpl]
impl MockVerifier {
    pub fn __constructor(e: &Env, mode: VerifierMode) {
        e.storage().instance().set(&symbol_short!("mode"), &mode);
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        match e.storage().instance().get(&symbol_short!("mode")).unwrap() {
            VerifierMode::Canonical => {
                Vec::from_iter(e, key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap()))
            }
            VerifierMode::Fail => {
                panic_with_error!(e, Error::from_contract_error(VERIFIER_FAILURE))
            }
            VerifierMode::CallPolicy(policy, guardian, smart_account) => {
                let signers = vec![e, Signer::Delegated(guardian.clone())];
                SocialRecoveryPolicyContractClient::new(e, &policy).approve(
                    &guardian,
                    &1,
                    &signers,
                    &RULE_ID,
                    &smart_account,
                );
                Vec::from_iter(e, key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap()))
            }
        }
    }
}

const START: u32 = 1_000_000;
const DELAY: u32 = 17_280;
const EXPIRY: u32 = 120_960;
const RULE_ID: u32 = 1;
const QUORUM: u32 = 1_010_000;
const EXECUTABLE_AFTER: u32 = 1_027_280;
const EXPIRES_AT: u32 = 1_148_240;

struct Fixture {
    e: Env,
    policy: Address,
    client: SocialRecoveryPolicyContractClient<'static>,
    account: Address,
    rule: ContextRule,
    bob: Address,
    carol: Address,
    dave: Address,
    mallory: Address,
    signers: Vec<Signer>,
}

impl Fixture {
    fn new() -> Self {
        let e = Env::default();
        e.mock_all_auths();
        e.ledger().with_mut(|li| {
            li.sequence_number = START;
            li.min_persistent_entry_ttl = 1_000_000;
        });
        let policy = e.register(SocialRecoveryPolicyContract, ());
        let client = SocialRecoveryPolicyContractClient::new(&e, &policy);
        let account = Address::generate(&e);
        let rule = ContextRule {
            id: RULE_ID,
            context_type: ContextRuleType::CallContract(account.clone()),
            name: String::from_str(&e, "recovery"),
            signers: Vec::new(&e),
            signer_ids: Vec::new(&e),
            policies: vec![&e, policy.clone()],
            policy_ids: vec![&e, 0],
            valid_until: None,
        };
        let verifier = e.register(MockVerifier, (VerifierMode::Canonical,));
        let signers = vec![&e, Signer::External(verifier, Bytes::from_array(&e, &[2; 32]))];
        Fixture {
            bob: Address::generate(&e),
            carol: Address::generate(&e),
            dave: Address::generate(&e),
            mallory: Address::generate(&e),
            e,
            policy,
            client,
            account,
            rule,
            signers,
        }
    }

    fn installed() -> Self {
        let f = Self::new();
        f.client.install(&f.params(), &f.rule, &f.account);
        f
    }

    /// Installed with Bob's approval at START and Carol's at QUORUM.
    fn ready() -> Self {
        let f = Self::installed();
        f.client.approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
        f.e.ledger().set_sequence_number(QUORUM);
        f.client.approve(&f.carol, &1, &f.signers, &RULE_ID, &f.account);
        f
    }

    fn params(&self) -> SocialRecoveryAccountParams {
        SocialRecoveryAccountParams {
            guardians: vec![&self.e, self.bob.clone(), self.carol.clone(), self.dave.clone()],
            threshold: 2,
            delay_ledgers: DELAY,
            expiry_ledgers: EXPIRY,
        }
    }

    fn completion(&self) -> Context {
        Context::Contract(ContractContext {
            contract: self.account.clone(),
            fn_name: Symbol::new(&self.e, "add_context_rule"),
            args: vec![
                &self.e,
                ContextRuleType::Default.into_val(&self.e),
                String::from_str(&self.e, RECOVERED_RULE_NAME).into_val(&self.e),
                None::<u32>.into_val(&self.e),
                self.signers.into_val(&self.e),
                Map::<Address, Val>::new(&self.e).into_val(&self.e),
            ],
        })
    }

    fn attempt(&self, epoch: u32) -> Option<RecoveryAttempt> {
        self.attempt_of(epoch, &self.signers)
    }

    fn attempt_of(&self, epoch: u32, signers: &Vec<Signer>) -> Option<RecoveryAttempt> {
        let hash = hash_signers(&self.e, signers);
        self.client.get_attempt(&epoch, &hash, &RULE_ID, &self.account)
    }

    /// Returns a one-key signer set on `verifier`.
    fn signers_on(&self, verifier: &Address) -> Vec<Signer> {
        vec![&self.e, Signer::External(verifier.clone(), Bytes::from_array(&self.e, &[3; 32]))]
    }

    /// Mocks `signer` authorizing exactly `fn_name(args)` on the policy.
    fn mock_auth(&self, signer: &Address, fn_name: &str, args: Vec<Val>) {
        self.e.mock_auths(&[MockAuth {
            address: signer,
            invoke: &MockAuthInvoke { contract: &self.policy, fn_name, args, sub_invokes: &[] },
        }]);
    }

    fn invocation(&self, fn_name: &str, args: Vec<Val>) -> AuthorizedInvocation {
        AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                self.policy.clone(),
                Symbol::new(&self.e, fn_name),
                args,
            )),
            sub_invocations: std::vec![],
        }
    }
}

fn contract_error(code: u32) -> Error {
    Error::from_contract_error(code)
}

/// The error a `try_*` client call returns when a host error, such as a
/// missing authorization, a rejected re-entry or a call to an address without
/// a contract, fails it. The host narrows every recoverable non-contract error
/// to this one at the call boundary; the `*_panics` tests pin the underlying
/// error.
fn host_error() -> Error {
    Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction)
}

#[test]
fn account_gated_functions_require_account_auth() {
    let f = Fixture::new();
    let params = f.params();

    f.client.install(&params, &f.rule, &f.account);
    assert_eq!(
        f.e.auths(),
        std::vec![(
            f.account.clone(),
            f.invocation(
                "install",
                (params.clone(), f.rule.clone(), f.account.clone()).into_val(&f.e)
            )
        )]
    );

    f.client.set_config(&params, &RULE_ID, &f.account);
    assert_eq!(
        f.e.auths(),
        std::vec![(
            f.account.clone(),
            f.invocation("set_config", (params.clone(), RULE_ID, f.account.clone()).into_val(&f.e))
        )]
    );

    f.client.cancel(&RULE_ID, &f.account);
    assert_eq!(
        f.e.auths(),
        std::vec![(
            f.account.clone(),
            f.invocation("cancel", (RULE_ID, f.account.clone()).into_val(&f.e))
        )]
    );

    f.client.uninstall(&f.rule, &f.account);
    assert_eq!(
        f.e.auths(),
        std::vec![(
            f.account.clone(),
            f.invocation("uninstall", (f.rule.clone(), f.account.clone()).into_val(&f.e))
        )]
    );
}

#[test]
fn account_gated_functions_reject_other_signer() {
    let f = Fixture::installed();
    let params = f.params();
    let mut second_rule = f.rule.clone();
    second_rule.id = 2;

    let args: Vec<Val> = (params.clone(), second_rule.clone(), f.account.clone()).into_val(&f.e);
    f.mock_auth(&f.mallory, "install", args);
    assert_eq!(f.client.try_install(&params, &second_rule, &f.account), Err(Ok(host_error())));

    let args: Vec<Val> = (params.clone(), RULE_ID, f.account.clone()).into_val(&f.e);
    f.mock_auth(&f.mallory, "set_config", args);
    assert_eq!(f.client.try_set_config(&params, &RULE_ID, &f.account), Err(Ok(host_error())));

    f.mock_auth(&f.mallory, "cancel", (RULE_ID, f.account.clone()).into_val(&f.e));
    assert_eq!(f.client.try_cancel(&RULE_ID, &f.account), Err(Ok(host_error())));

    f.mock_auth(&f.mallory, "uninstall", (f.rule.clone(), f.account.clone()).into_val(&f.e));
    assert_eq!(f.client.try_uninstall(&f.rule, &f.account), Err(Ok(host_error())));
    assert!(f.e.events().all().events().is_empty());

    assert_eq!(f.client.get_epoch(&RULE_ID, &f.account), 1);
    assert_eq!(f.client.get_epoch(&2, &f.account), 0);
    assert_eq!(f.client.get_config(&RULE_ID, &f.account), params);
}

#[test]
fn approve_requires_only_guardian_auth() {
    let f = Fixture::installed();
    f.client.approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
    let args = (f.bob.clone(), 1u32, f.signers.clone(), RULE_ID, f.account.clone());
    assert_eq!(
        f.e.auths(),
        std::vec![(f.bob.clone(), f.invocation("approve", args.into_val(&f.e)))]
    );
}

#[test]
fn revoke_approval_requires_only_guardian_auth() {
    let f = Fixture::installed();
    f.client.approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
    f.client.revoke_approval(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
    let args = (f.bob.clone(), 1u32, f.signers.clone(), RULE_ID, f.account.clone());
    assert_eq!(
        f.e.auths(),
        std::vec![(f.bob.clone(), f.invocation("revoke_approval", args.into_val(&f.e)))]
    );
}

#[test]
fn approve_signed_by_other_guardian_fails() {
    let f = Fixture::installed();
    let args = (f.bob.clone(), 1u32, f.signers.clone(), RULE_ID, f.account.clone());
    f.mock_auth(&f.carol, "approve", args.into_val(&f.e));
    assert_eq!(
        f.client.try_approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account),
        Err(Ok(host_error()))
    );
    assert_eq!(f.attempt(1), None);
}

#[test]
fn approve_auth_binds_every_argument() {
    let f = Fixture::installed();
    let mut second_rule = f.rule.clone();
    second_rule.id = 2;
    f.client.install(&f.params(), &second_rule, &f.account);
    let other_account = Address::generate(&f.e);
    let other_signers =
        vec![&f.e, Signer::External(Address::generate(&f.e), Bytes::from_array(&f.e, &[3; 32]))];
    let signed = (f.bob.clone(), 1u32, f.signers.clone(), RULE_ID, f.account.clone());

    let calls = [
        ("guardian", f.carol.clone(), 1u32, f.signers.clone(), RULE_ID, f.account.clone()),
        ("epoch", f.bob.clone(), 2, f.signers.clone(), RULE_ID, f.account.clone()),
        ("signers", f.bob.clone(), 1, other_signers, RULE_ID, f.account.clone()),
        ("context_rule_id", f.bob.clone(), 1, f.signers.clone(), 2, f.account.clone()),
        ("smart_account", f.bob.clone(), 1, f.signers.clone(), RULE_ID, other_account),
    ];
    for (changed, guardian, epoch, signers, rule_id, account) in calls {
        f.mock_auth(&f.bob, "approve", signed.clone().into_val(&f.e));
        assert_eq!(
            f.client.try_approve(&guardian, &epoch, &signers, &rule_id, &account),
            Err(Ok(host_error())),
            "call with a different {changed} must fail"
        );
    }
    assert_eq!(f.attempt(1), None);
}

#[test]
fn enforce_without_account_auth_fails() {
    let f = Fixture::ready();
    f.e.ledger().set_sequence_number(EXECUTABLE_AFTER);
    f.e.set_auths(&[]);
    assert_eq!(
        f.client.try_enforce(&f.completion(), &Vec::new(&f.e), &f.rule, &f.account),
        Err(Ok(host_error()))
    );
    assert!(f.e.events().all().events().is_empty());
    assert_eq!(f.client.get_epoch(&RULE_ID, &f.account), 1);
    assert!(f.attempt(1).is_some());
}

#[test]
fn direct_enforce_with_account_auth_only_retires_attempt() {
    let f = Fixture::ready();
    f.e.ledger().set_sequence_number(EXECUTABLE_AFTER);
    f.client.enforce(&f.completion(), &Vec::new(&f.e), &f.rule, &f.account);

    let auths = f.e.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, f.account);
    let events = f.e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &RecoveryAuthorized {
            smart_account: f.account.clone(),
            context_rule_id: RULE_ID,
            epoch: 1,
            signers_hash: hash_signers(&f.e, &f.signers),
        }
        .to_xdr(&f.e, &f.policy)
    );
    assert_eq!(f.client.get_epoch(&RULE_ID, &f.account), 2);
    assert_eq!(f.attempt(1), None);
}

#[test]
fn outsiders_cannot_disturb_ready_attempt() {
    let f = Fixture::ready();
    let before = f.attempt(1);
    assert_eq!(before.as_ref().unwrap().executable_after, Some(EXECUTABLE_AFTER));

    assert_eq!(
        f.client.try_approve(&f.mallory, &1, &f.signers, &RULE_ID, &f.account),
        Err(Ok(contract_error(3234))) // SocialRecoveryError::NotGuardian
    );
    assert_eq!(
        f.client.try_revoke_approval(&f.mallory, &1, &f.signers, &RULE_ID, &f.account),
        Err(Ok(contract_error(3234))) // SocialRecoveryError::NotGuardian
    );
    assert_eq!(
        f.client.try_revoke_approval(&f.dave, &1, &f.signers, &RULE_ID, &f.account),
        Err(Ok(contract_error(3237))) // SocialRecoveryError::ApprovalNotFound
    );
    f.mock_auth(&f.mallory, "cancel", (RULE_ID, f.account.clone()).into_val(&f.e));
    assert_eq!(f.client.try_cancel(&RULE_ID, &f.account), Err(Ok(host_error())));
    assert!(f.e.events().all().events().is_empty());

    assert_eq!(f.attempt(1), before);
    assert_eq!(f.client.get_epoch(&RULE_ID, &f.account), 1);
}

#[test]
fn view_state_matches_enforce_at_each_boundary() {
    // Independent oracle: the predicate for an executable attempt.
    fn executable(attempt: &RecoveryAttempt, expiry: u32, ledger: u32) -> bool {
        attempt.executable_after.is_some_and(|ea| ea <= ledger && ledger < ea + expiry)
    }

    let boundaries = [
        (EXECUTABLE_AFTER - 1, false),
        (EXECUTABLE_AFTER, true),
        (EXPIRES_AT - 1, true),
        (EXPIRES_AT, false),
    ];
    for (ledger, expected) in boundaries {
        let f = Fixture::ready();
        f.e.ledger().set_sequence_number(ledger);
        let config = f.client.get_config(&RULE_ID, &f.account);
        let derived = executable(&f.attempt(1).unwrap(), config.expiry_ledgers, ledger);
        assert_eq!(derived, expected, "view-derived state at {ledger}");
        let outcome = f.client.try_enforce(&f.completion(), &Vec::new(&f.e), &f.rule, &f.account);
        assert_eq!(outcome.is_ok(), expected, "enforce outcome at {ledger}");
        if !expected {
            assert_eq!(outcome, Err(Ok(contract_error(3238)))); // SocialRecoveryError::NotAllowed
        }
    }
}

#[test]
fn failed_call_emits_no_events() {
    let f = Fixture::ready();
    assert_eq!(
        f.client.try_approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account),
        Err(Ok(contract_error(3236))) // SocialRecoveryError::AlreadyApproved
    );
    assert!(f.e.events().all().events().is_empty());
}

#[test]
fn unusable_verifiers_fail_approval() {
    let f = Fixture::installed();
    let failing = f.e.register(MockVerifier, (VerifierMode::Fail,));
    let mode = VerifierMode::CallPolicy(f.policy.clone(), f.dave.clone(), f.account.clone());
    let calling_policy = f.e.register(MockVerifier, (mode,));
    let cases = [
        ("failing verifier", failing, contract_error(VERIFIER_FAILURE)),
        ("policy as verifier", f.policy.clone(), host_error()),
        ("address without a contract", Address::generate(&f.e), host_error()),
        ("verifier calling the policy", calling_policy, host_error()),
    ];
    for (case, verifier, error) in cases {
        let signers = f.signers_on(&verifier);
        assert_eq!(
            f.client.try_approve(&f.bob, &1, &signers, &RULE_ID, &f.account),
            Err(Ok(error)),
            "{case}"
        );
        assert!(f.e.events().all().events().is_empty(), "{case}");
        assert_eq!(f.attempt_of(1, &signers), None, "{case}");
    }

    f.client.approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
    assert_eq!(f.attempt(1).unwrap().approvals, vec![&f.e, f.bob.clone()]);
}

#[test]
fn failing_verifier_blocks_only_its_signer_sets() {
    let f = Fixture::installed();
    let failing = f.e.register(MockVerifier, (VerifierMode::Fail,));
    let mut mixed = f.signers.clone();
    mixed.append(&f.signers_on(&failing));
    assert_eq!(
        f.client.try_approve(&f.bob, &1, &mixed, &RULE_ID, &f.account),
        Err(Ok(contract_error(VERIFIER_FAILURE)))
    );
    assert_eq!(f.attempt_of(1, &mixed), None);

    f.client.approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
    f.client.approve(&f.carol, &1, &f.signers, &RULE_ID, &f.account);
    assert_eq!(f.attempt(1).unwrap().executable_after, Some(START + DELAY));
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn enforce_without_account_auth_panics() {
    let f = Fixture::ready();
    f.e.ledger().set_sequence_number(EXECUTABLE_AFTER);
    f.e.set_auths(&[]);
    f.client.enforce(&f.completion(), &Vec::new(&f.e), &f.rule, &f.account);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn cancel_signed_by_other_address_panics() {
    let f = Fixture::installed();
    f.mock_auth(&f.mallory, "cancel", (RULE_ID, f.account.clone()).into_val(&f.e));
    f.client.cancel(&RULE_ID, &f.account);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn approve_signed_by_other_guardian_panics() {
    let f = Fixture::installed();
    let args = (f.bob.clone(), 1u32, f.signers.clone(), RULE_ID, f.account.clone());
    f.mock_auth(&f.carol, "approve", args.into_val(&f.e));
    f.client.approve(&f.bob, &1, &f.signers, &RULE_ID, &f.account);
}

#[test]
#[should_panic(expected = "Contract re-entry is not allowed")]
fn policy_as_verifier_panics() {
    let f = Fixture::installed();
    f.client.approve(&f.bob, &1, &f.signers_on(&f.policy), &RULE_ID, &f.account);
}

#[test]
#[should_panic(expected = "Contract re-entry is not allowed")]
fn verifier_calling_policy_panics() {
    let f = Fixture::installed();
    let mode = VerifierMode::CallPolicy(f.policy.clone(), f.dave.clone(), f.account.clone());
    let verifier = f.e.register(MockVerifier, (mode,));
    f.client.approve(&f.bob, &1, &f.signers_on(&verifier), &RULE_ID, &f.account);
}

#[test]
#[should_panic(expected = "HostError: Error(Storage, MissingValue)")]
fn verifier_without_contract_panics() {
    let f = Fixture::installed();
    let signers = f.signers_on(&Address::generate(&f.e));
    f.client.approve(&f.bob, &1, &signers, &RULE_ID, &f.account);
}
