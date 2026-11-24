extern crate std;

use core::cell::Cell;

use soroban_sdk::{
    auth::Context,
    contract, contractimpl, map,
    testutils::{Address as _, Events, Ledger},
    vec,
    xdr::{
        InvokeContractArgs, ScVal, SorobanAddressCredentials, SorobanAuthorizationEntry,
        SorobanAuthorizedFunction, SorobanAuthorizedInvocation, SorobanCredentials, VecM,
    },
    Address, Bytes, BytesN, Env, Event, IntoVal, Map, String, TryFromVal, Val, Vec,
};
use stellar_accounts::{
    policies::{
        simple_threshold::{self, SimpleThresholdAccountParams},
        social_recovery::{
            self, hash_signers, RecoveryAttempt, RecoveryAuthorized, RecoveryTemplate,
            SocialRecoveryAccountParams,
        },
        spending_limit::{self, SpendingLimitAccountParams},
        Policy,
    },
    smart_account::{
        AuthPayload, ContextRule, ContextRuleAdded, ContextRuleType, Signer, SignerRegistered,
        MAX_EXTERNAL_KEY_SIZE, MAX_SIGNERS,
    },
};
use stellar_test_utils::assert_events_and_return_within;

use crate::contract::{MultisigContract, MultisigContractClient};

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type AccountParams = Val;

    fn enforce(
        _e: &Env,
        _context: soroban_sdk::auth::Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(
        _e: &Env,
        _install_params: Self::AccountParams,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(_e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        true
    }

    pub fn canonicalize_key(e: &Env, key_data: Val) -> Bytes {
        Bytes::try_from_val(e, &key_data).unwrap()
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(e, key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap()))
    }
}

#[test]
fn create_account() {
    let e = Env::default();
    let verifier = e.register(MockVerifierContract, ());
    let policy = e.register(MockPolicyContract, ());
    let signers = vec![
        &e,
        Signer::External(
            verifier.clone(),
            Bytes::from_array(
                &e,
                b"4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29",
            ),
        ),
        Signer::External(
            verifier.clone(),
            Bytes::from_array(
                &e,
                b"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
            ),
        ),
    ];
    let policies = map![&e, (policy, SimpleThresholdAccountParams { threshold: 2 })];
    e.register(MultisigContract, (String::from_str(&e, "multisig"), signers, policies));
}

#[test]
fn batch_add_signer_adds_all_signers() {
    let e = Env::default();
    e.mock_all_auths();

    let verifier = e.register(MockVerifierContract, ());
    let initial_signer = Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, b"4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29"),
    );
    let account = e.register(
        MultisigContract,
        (
            String::from_str(&e, "multisig"),
            vec![&e, initial_signer.clone()],
            soroban_sdk::Map::<Address, Val>::new(&e),
        ),
    );

    let new_signer_1 = Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, b"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"),
    );
    let new_signer_2 = Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, b"1111111111111111111111111111111111111111111111111111111111111111"),
    );

    let client = crate::contract::MultisigContractClient::new(&e, &account);
    client.batch_add_signer(&0, &vec![&e, new_signer_1.clone(), new_signer_2.clone()]);

    let rule = client.get_context_rule(&0);
    assert_eq!(rule.signers.len(), 3);
    assert!(rule.signers.contains(&initial_signer));
    assert!(rule.signers.contains(&new_signer_1));
    assert!(rule.signers.contains(&new_signer_2));
}

// ################## SOCIAL RECOVERY ##################
//
// The recovery, threshold and spending-limit policies are inline wrappers with
// the same dispatch as their example contracts, which are `cdylib`-only and
// cannot be linked here. The account is the unmodified `MultisigContract`, and
// completions go through its real `__check_auth`.

#[contract]
struct RecoveryPolicyContract;

#[contractimpl]
impl Policy for RecoveryPolicyContract {
    type AccountParams = SocialRecoveryAccountParams;

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        social_recovery::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        social_recovery::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        social_recovery::uninstall(e, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl RecoveryPolicyContract {
    pub fn approve(
        e: &Env,
        guardian: Address,
        epoch: u32,
        signers: Vec<Signer>,
        context_rule_id: u32,
        smart_account: Address,
    ) {
        social_recovery::approve(e, &guardian, epoch, &signers, context_rule_id, &smart_account)
    }

    pub fn cancel(e: &Env, context_rule_id: u32, smart_account: Address) {
        social_recovery::cancel(e, context_rule_id, &smart_account)
    }

    pub fn set_config(
        e: &Env,
        params: SocialRecoveryAccountParams,
        context_rule_id: u32,
        smart_account: Address,
    ) {
        social_recovery::set_config(e, &params, context_rule_id, &smart_account)
    }

    pub fn get_epoch(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        social_recovery::get_epoch(e, context_rule_id, &smart_account)
    }

    pub fn get_attempt(
        e: &Env,
        epoch: u32,
        signers_hash: BytesN<32>,
        context_rule_id: u32,
        smart_account: Address,
    ) -> Option<RecoveryAttempt> {
        social_recovery::get_attempt(e, epoch, &signers_hash, context_rule_id, &smart_account)
    }
}

#[contract]
struct ThresholdPolicyContract;

#[contractimpl]
impl Policy for ThresholdPolicyContract {
    type AccountParams = SimpleThresholdAccountParams;

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::enforce(
            e,
            &context,
            &authenticated_signers,
            &context_rule,
            &smart_account,
        )
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        simple_threshold::uninstall(e, &context_rule, &smart_account)
    }
}

#[contract]
struct SpendingLimitPolicyContract;

#[contractimpl]
impl Policy for SpendingLimitPolicyContract {
    type AccountParams = SpendingLimitAccountParams;

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        spending_limit::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        spending_limit::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        spending_limit::uninstall(e, &context_rule, &smart_account)
    }
}

/// Submits a completion from a contract that catches its failure.
#[contract]
struct RouterContract;

#[contractimpl]
impl RouterContract {
    pub fn try_recover(
        e: &Env,
        account: Address,
        template: RecoveryTemplate,
        signers: Vec<Signer>,
    ) -> bool {
        MultisigContractClient::new(e, &account)
            .try_add_context_rule(
                &template.context_type,
                &template.name,
                &None,
                &signers,
                &template.policies,
            )
            .is_ok()
    }
}

const START: u32 = 1_000_000;
const DELAY: u32 = 17_280;
const EXPIRY: u32 = 120_960;
const EXECUTABLE_AFTER: u32 = START + DELAY;

fn to_scval<T: IntoVal<Env, Val>>(e: &Env, value: T) -> ScVal {
    ScVal::try_from_val(e, &value.into_val(e)).unwrap()
}

fn contract_fn(contract: &Address, name: &str, args: &[ScVal]) -> SorobanAuthorizedInvocation {
    SorobanAuthorizedInvocation {
        function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
            contract_address: contract.into(),
            function_name: name.try_into().unwrap(),
            args: args.to_vec().try_into().unwrap(),
        }),
        sub_invocations: VecM::default(),
    }
}

struct Recovery {
    e: Env,
    account: Address,
    client: MultisigContractClient<'static>,
    policy: Address,
    policy_client: RecoveryPolicyContractClient<'static>,
    verifier: Address,
    owner: Signer,
    bob: Address,
    carol: Address,
    dave: Address,
    rule_id: Cell<u32>,
    nonce: Cell<i64>,
}

impl Recovery {
    /// An account with an owner rule (ID 0) and no recovery rule.
    fn bare() -> Self {
        let e = Env::default();
        e.mock_all_auths();
        e.ledger().with_mut(|li| {
            li.sequence_number = START;
            li.min_persistent_entry_ttl = 1_000_000;
        });
        let verifier = e.register(MockVerifierContract, ());
        let owner = Signer::External(verifier.clone(), Bytes::from_array(&e, &[1; 32]));
        let account = e.register(
            MultisigContract,
            (String::from_str(&e, "owner"), vec![&e, owner.clone()], Map::<Address, Val>::new(&e)),
        );
        let policy = e.register(RecoveryPolicyContract, ());
        Recovery {
            client: MultisigContractClient::new(&e, &account),
            policy_client: RecoveryPolicyContractClient::new(&e, &policy),
            bob: Address::generate(&e),
            carol: Address::generate(&e),
            dave: Address::generate(&e),
            rule_id: Cell::new(u32::MAX),
            nonce: Cell::new(0),
            e,
            account,
            policy,
            verifier,
            owner,
        }
    }

    /// An account with an owner rule and a recovery rule using the default
    /// template.
    fn new() -> Self {
        let r = Self::bare();
        r.add_recovery_rule(&r.params(r.template(Map::new(&r.e))), None);
        r
    }

    fn template(&self, policies: Map<Address, Val>) -> RecoveryTemplate {
        RecoveryTemplate {
            context_type: ContextRuleType::Default,
            name: String::from_str(&self.e, "recovered"),
            policies,
        }
    }

    fn params(&self, template: RecoveryTemplate) -> SocialRecoveryAccountParams {
        SocialRecoveryAccountParams {
            guardians: vec![&self.e, self.bob.clone(), self.carol.clone(), self.dave.clone()],
            threshold: 2,
            delay_ledgers: DELAY,
            expiry_ledgers: EXPIRY,
            template,
        }
    }

    fn add_recovery_rule(
        &self,
        params: &SocialRecoveryAccountParams,
        extra_policy: Option<(Address, Val)>,
    ) -> u32 {
        self.e.mock_all_auths();
        let mut policies = map![&self.e, (self.policy.clone(), params.into_val(&self.e))];
        if let Some((policy, install_param)) = extra_policy {
            policies.set(policy, install_param);
        }
        let rule = self.client.add_context_rule(
            &ContextRuleType::CallContract(self.account.clone()),
            &String::from_str(&self.e, "recovery"),
            &None,
            &Vec::new(&self.e),
            &policies,
        );
        self.rule_id.set(rule.id);
        rule.id
    }

    fn signers(&self, keys: &[u8]) -> Vec<Signer> {
        Vec::from_iter(
            &self.e,
            keys.iter().map(|key| {
                Signer::External(self.verifier.clone(), Bytes::from_array(&self.e, &[*key; 32]))
            }),
        )
    }

    fn at(&self, ledger: u32) {
        self.e.ledger().set_sequence_number(ledger);
    }

    fn epoch(&self) -> u32 {
        self.policy_client.get_epoch(&self.rule_id.get(), &self.account)
    }

    fn attempt(&self, epoch: u32, signers: &Vec<Signer>) -> Option<RecoveryAttempt> {
        let hash = hash_signers(&self.e, signers);
        self.policy_client.get_attempt(&epoch, &hash, &self.rule_id.get(), &self.account)
    }

    fn approve(&self, guardian: &Address, signers: &Vec<Signer>) {
        self.e.mock_all_auths();
        let epoch = self.epoch();
        self.policy_client.approve(guardian, &epoch, signers, &self.rule_id.get(), &self.account);
    }

    /// Bob and Carol approve `signers` at START, so the attempt is executable
    /// from EXECUTABLE_AFTER.
    fn reach_quorum(&self, signers: &Vec<Signer>) {
        self.at(START);
        self.approve(&self.bob, signers);
        self.approve(&self.carol, signers);
    }

    /// Returns an authorization entry for `address` whose `AuthPayload` holds
    /// `signatures` and selects `context_rule_ids`.
    fn account_entry(
        &self,
        address: &Address,
        signatures: Map<Signer, Bytes>,
        context_rule_ids: Vec<u32>,
        root_invocation: SorobanAuthorizedInvocation,
    ) -> SorobanAuthorizationEntry {
        let nonce = self.nonce.get() + 1;
        self.nonce.set(nonce);
        let payload = AuthPayload { signers: signatures, context_rule_ids };
        SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: address.into(),
                nonce,
                signature_expiration_ledger: self.e.ledger().sequence() + 100,
                signature: to_scval(&self.e, payload),
            }),
            root_invocation,
        }
    }

    /// Authorizes a completion with no signatures through the recovery rule.
    fn authorize_completion(&self, template: &RecoveryTemplate, signers: &Vec<Signer>) {
        let invocation = contract_fn(
            &self.account,
            "add_context_rule",
            &[
                to_scval(&self.e, template.context_type.clone()),
                to_scval(&self.e, template.name.clone()),
                to_scval(&self.e, None::<u32>),
                to_scval(&self.e, signers.clone()),
                to_scval(&self.e, template.policies.clone()),
            ],
        );
        let rule_ids = vec![&self.e, self.rule_id.get()];
        let entry = self.account_entry(&self.account, Map::new(&self.e), rule_ids, invocation);
        self.e.set_auths(&[entry]);
    }

    fn complete(&self, template: &RecoveryTemplate, signers: &Vec<Signer>) -> ContextRule {
        self.authorize_completion(template, signers);
        self.client.add_context_rule(
            &template.context_type,
            &template.name,
            &None,
            signers,
            &template.policies,
        )
    }

    fn try_complete(&self, template: &RecoveryTemplate, signers: &Vec<Signer>) -> bool {
        self.authorize_completion(template, signers);
        self.client
            .try_add_context_rule(
                &template.context_type,
                &template.name,
                &None,
                signers,
                &template.policies,
            )
            .is_ok()
    }

    /// Authorizes `cancel` on the policy with `signatures` selecting
    /// `context_rule_id`.
    fn authorize_cancel(&self, signatures: Map<Signer, Bytes>, context_rule_id: u32) {
        let invocation = contract_fn(
            &self.policy,
            "cancel",
            &[to_scval(&self.e, self.rule_id.get()), to_scval(&self.e, self.account.clone())],
        );
        let rule_ids = vec![&self.e, context_rule_id];
        let entry = self.account_entry(&self.account, signatures, rule_ids, invocation);
        self.e.set_auths(&[entry]);
    }
}

#[test]
fn recovery_completes_through_account() {
    let r = Recovery::new();
    let rule_id = r.rule_id.get();
    let owner_rule = r.client.get_context_rule(&0);
    let recovery_rule = r.client.get_context_rule(&rule_id);
    let count = r.client.get_context_rules_count();
    let signers = r.signers(&[2]);
    let template = r.template(Map::new(&r.e));
    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);

    let rule = r.complete(&template, &signers);

    let events = r.e.events().all();
    assert_eq!(events.events().len(), 3);
    assert_eq!(
        events.events().first().unwrap(),
        &RecoveryAuthorized {
            smart_account: r.account.clone(),
            context_rule_id: rule_id,
            epoch: 1,
            signers_hash: hash_signers(&r.e, &signers),
        }
        .to_xdr(&r.e, &r.policy)
    );
    assert_eq!(
        events.events().get(1).unwrap(),
        &SignerRegistered {
            signer_id: rule.signer_ids.get(0).unwrap(),
            signer: signers.get(0).unwrap(),
        }
        .to_xdr(&r.e, &r.account)
    );
    assert_eq!(
        events.events().get(2).unwrap(),
        &ContextRuleAdded {
            context_rule_id: rule.id,
            name: template.name.clone(),
            context_type: template.context_type.clone(),
            valid_until: None,
            signer_ids: rule.signer_ids.clone(),
            policy_ids: Vec::new(&r.e),
        }
        .to_xdr(&r.e, &r.account)
    );

    assert_eq!(rule.context_type, ContextRuleType::Default);
    assert_eq!(rule.name, template.name);
    assert_eq!(rule.valid_until, None);
    assert_eq!(rule.signers, signers);
    assert!(rule.policies.is_empty());
    assert_eq!(r.client.get_context_rules_count(), count + 1);
    assert_eq!(r.client.get_context_rule(&0), owner_rule);
    assert_eq!(r.client.get_context_rule(&rule_id), recovery_rule);
    assert_eq!(r.client.get_context_rule(&rule.id), rule);
    assert_eq!(r.epoch(), 2);
    assert_eq!(r.attempt(1, &signers), None);
}

#[test]
fn recovery_rule_valid_until_ledger_still_completes() {
    let r = Recovery::bare();
    let params = r.params(r.template(Map::new(&r.e)));
    let rule = r.client.add_context_rule(
        &ContextRuleType::CallContract(r.account.clone()),
        &String::from_str(&r.e, "recovery"),
        &Some(EXECUTABLE_AFTER),
        &Vec::new(&r.e),
        &map![&r.e, (r.policy.clone(), params.into_val(&r.e))],
    );
    r.rule_id.set(rule.id);
    let signers = r.signers(&[2]);
    r.reach_quorum(&signers);

    r.at(EXECUTABLE_AFTER + 1);
    assert!(!r.try_complete(&params.template, &signers));
    r.at(EXECUTABLE_AFTER);
    assert!(r.try_complete(&params.template, &signers));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn completion_below_threshold_panics() {
    let r = Recovery::new();
    let signers = r.signers(&[2]);
    r.approve(&r.bob, &signers);
    r.at(START + EXPIRY - 1);
    r.complete(&r.template(Map::new(&r.e)), &signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn completion_with_reordered_signers_panics() {
    let r = Recovery::new();
    r.reach_quorum(&r.signers(&[2, 3]));
    r.at(EXECUTABLE_AFTER);
    r.complete(&r.template(Map::new(&r.e)), &r.signers(&[3, 2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn completion_replay_through_account_panics() {
    let r = Recovery::new();
    let signers = r.signers(&[2]);
    let template = r.template(Map::new(&r.e));
    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);
    r.complete(&template, &signers);
    r.complete(&template, &signers);
}

#[test]
fn failed_completion_consumes_nothing() {
    let r = Recovery::bare();
    let threshold = r.e.register(ThresholdPolicyContract, ());
    let policies =
        map![&r.e, (threshold, SimpleThresholdAccountParams { threshold: 2 }.into_val(&r.e))];
    let template = r.template(policies);
    r.add_recovery_rule(&r.params(template.clone()), None);
    let signers = r.signers(&[2]);
    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);
    let before = r.attempt(1, &signers);

    r.authorize_completion(&template, &signers);
    let result = r.client.try_add_context_rule(
        &template.context_type,
        &template.name,
        &None,
        &signers,
        &template.policies,
    );
    // SimpleThresholdError::InvalidThreshold
    assert_eq!(result.err(), Some(Ok(soroban_sdk::Error::from_contract_error(3201))));
    assert!(r.e.events().all().events().is_empty());
    assert_eq!(r.epoch(), 1);
    assert_eq!(r.attempt(1, &signers), before);
}

#[test]
fn caught_failed_completion_consumes_nothing() {
    let r = Recovery::bare();
    let threshold = r.e.register(ThresholdPolicyContract, ());
    let router = RouterContractClient::new(&r.e, &r.e.register(RouterContract, ()));
    let policies =
        map![&r.e, (threshold, SimpleThresholdAccountParams { threshold: 2 }.into_val(&r.e))];
    let template = r.template(policies);
    r.add_recovery_rule(&r.params(template.clone()), None);
    let signers = r.signers(&[2]);
    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);
    let before = r.attempt(1, &signers);

    r.authorize_completion(&template, &signers);
    assert!(!router.try_recover(&r.account, &template, &signers));
    assert!(r.e.events().all().events().is_empty());
    assert_eq!(r.epoch(), 1);
    assert_eq!(r.attempt(1, &signers), before);
}

#[test]
fn remove_and_readd_policy_keeps_epoch_monotonic() {
    let r = Recovery::bare();
    let other = r.e.register(MockPolicyContract, ());
    let params = r.params(r.template(Map::new(&r.e)));
    let rule_id = r.add_recovery_rule(&params, Some((other, 0u32.into_val(&r.e))));
    assert_eq!(r.epoch(), 1);

    let rule = r.client.get_context_rule(&rule_id);
    let index = rule.policies.first_index_of(&r.policy).unwrap();
    r.client.remove_policy(&rule_id, &rule.policy_ids.get(index).unwrap());
    assert_eq!(r.epoch(), 1);
    r.client.add_policy(&rule_id, &r.policy, &params.into_val(&r.e));
    assert_eq!(r.epoch(), 2);

    let signers = r.signers(&[2]);
    assert_eq!(
        r.policy_client.try_approve(&r.bob, &1, &signers, &rule_id, &r.account),
        Err(Ok(soroban_sdk::Error::from_contract_error(3235))) // SocialRecoveryError::StaleEpoch
    );
    r.policy_client.cancel(&rule_id, &r.account);
    assert_eq!(r.epoch(), 3);

    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);
    r.complete(&params.template, &signers);
    assert_eq!(r.epoch(), 4);
}

#[test]
fn ready_attempt_does_not_survive_reinstall() {
    let r = Recovery::bare();
    let other = r.e.register(MockPolicyContract, ());
    let params = r.params(r.template(Map::new(&r.e)));
    let rule_id = r.add_recovery_rule(&params, Some((other, 0u32.into_val(&r.e))));
    let signers = r.signers(&[2]);
    r.reach_quorum(&signers);

    r.e.mock_all_auths();
    let rule = r.client.get_context_rule(&rule_id);
    let index = rule.policies.first_index_of(&r.policy).unwrap();
    r.client.remove_policy(&rule_id, &rule.policy_ids.get(index).unwrap());
    r.client.add_policy(&rule_id, &r.policy, &params.into_val(&r.e));
    assert_eq!(r.epoch(), 2);

    r.at(EXECUTABLE_AFTER);
    let count = r.client.get_context_rules_count();
    assert!(!r.try_complete(&params.template, &signers));
    assert_eq!(r.client.get_context_rules_count(), count);

    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);
    assert!(r.try_complete(&params.template, &signers));
    assert_eq!(r.client.get_context_rules_count(), count + 1);
    assert_eq!(r.epoch(), 3);
}

#[test]
fn owner_rule_can_cancel() {
    let r = Recovery::new();
    r.authorize_cancel(map![&r.e, (r.owner.clone(), Bytes::from_array(&r.e, &[9; 64]))], 0);
    r.policy_client.cancel(&r.rule_id.get(), &r.account);
    assert_eq!(r.epoch(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")] // SmartAccountError::UnvalidatedContext
fn spending_limit_session_key_on_token_cannot_cancel() {
    let r = Recovery::new();
    let spending_limit = r.e.register(SpendingLimitPolicyContract, ());
    let session = Signer::External(r.verifier.clone(), Bytes::from_array(&r.e, &[5; 32]));
    let limit = SpendingLimitAccountParams { spending_limit: 1_000, period_ledgers: DELAY };
    let session_rule = r.client.add_context_rule(
        &ContextRuleType::CallContract(Address::generate(&r.e)),
        &String::from_str(&r.e, "session"),
        &None,
        &vec![&r.e, session.clone()],
        &map![&r.e, (spending_limit, limit.into_val(&r.e))],
    );

    r.authorize_cancel(map![&r.e, (session, Bytes::from_array(&r.e, &[9; 64]))], session_rule.id);
    r.policy_client.cancel(&r.rule_id.get(), &r.account);
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")] // SpendingLimitError::NotAllowed
fn spending_limit_rule_on_policy_cannot_cancel() {
    let r = Recovery::new();
    let spending_limit = r.e.register(SpendingLimitPolicyContract, ());
    let session = Signer::External(r.verifier.clone(), Bytes::from_array(&r.e, &[5; 32]));
    let limit = SpendingLimitAccountParams { spending_limit: 1_000, period_ledgers: DELAY };
    let session_rule = r.client.add_context_rule(
        &ContextRuleType::CallContract(r.policy.clone()),
        &String::from_str(&r.e, "session"),
        &None,
        &vec![&r.e, session.clone()],
        &map![&r.e, (spending_limit, limit.into_val(&r.e))],
    );

    r.authorize_cancel(map![&r.e, (session, Bytes::from_array(&r.e, &[9; 64]))], session_rule.id);
    r.policy_client.cancel(&r.rule_id.get(), &r.account);
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")] // SmartAccountError::UnvalidatedContext
fn recovery_rule_cannot_authorize_cancel() {
    let r = Recovery::new();
    r.authorize_cancel(Map::new(&r.e), r.rule_id.get());
    r.policy_client.cancel(&r.rule_id.get(), &r.account);
}

#[test]
fn policy_contract_rule_can_take_over() {
    let r = Recovery::new();
    let key = Signer::External(r.verifier.clone(), Bytes::from_array(&r.e, &[5; 32]));
    let key_rule = r.client.add_context_rule(
        &ContextRuleType::CallContract(r.policy.clone()),
        &String::from_str(&r.e, "canceller"),
        &None,
        &vec![&r.e, key.clone()],
        &Map::new(&r.e),
    );
    let attacker = Address::generate(&r.e);
    let mut params = r.params(r.template(Map::new(&r.e)));
    params.guardians = vec![&r.e, attacker.clone()];
    params.threshold = 1;
    params.delay_ledgers = 1;
    let invocation = contract_fn(
        &r.policy,
        "set_config",
        &[
            to_scval(&r.e, params.clone()),
            to_scval(&r.e, r.rule_id.get()),
            to_scval(&r.e, r.account.clone()),
        ],
    );
    let signatures = map![&r.e, (key, Bytes::from_array(&r.e, &[9; 64]))];
    let entry = r.account_entry(&r.account, signatures, vec![&r.e, key_rule.id], invocation);
    r.e.set_auths(&[entry]);
    r.policy_client.set_config(&params, &r.rule_id.get(), &r.account);

    let signers = r.signers(&[6]);
    r.approve(&attacker, &signers);
    r.at(START + 1);
    let rule = r.complete(&params.template, &signers);

    assert_eq!(rule.context_type, ContextRuleType::Default);
    assert_eq!(rule.signers, signers);
}

#[test]
fn smart_account_guardian_approves_through_check_auth() {
    let r = Recovery::bare();
    let guardian_owner = Signer::External(r.verifier.clone(), Bytes::from_array(&r.e, &[7; 32]));
    let guardian = r.e.register(
        MultisigContract,
        (
            String::from_str(&r.e, "guardian"),
            vec![&r.e, guardian_owner.clone()],
            Map::<Address, Val>::new(&r.e),
        ),
    );
    let mut params = r.params(r.template(Map::new(&r.e)));
    params.guardians = vec![&r.e, guardian.clone(), r.carol.clone()];
    let rule_id = r.add_recovery_rule(&params, None);
    let signers = r.signers(&[2]);

    let invocation = contract_fn(
        &r.policy,
        "approve",
        &[
            to_scval(&r.e, guardian.clone()),
            to_scval(&r.e, 1u32),
            to_scval(&r.e, signers.clone()),
            to_scval(&r.e, rule_id),
            to_scval(&r.e, r.account.clone()),
        ],
    );
    let entry = r.account_entry(
        &guardian,
        map![&r.e, (guardian_owner, Bytes::from_array(&r.e, &[9; 64]))],
        vec![&r.e, 0],
        invocation,
    );
    r.e.set_auths(&[entry]);
    r.policy_client.approve(&guardian, &1, &signers, &rule_id, &r.account);

    assert_eq!(r.attempt(1, &signers).unwrap().approvals, vec![&r.e, guardian]);
}

#[test]
fn max_signer_completion_events_fit() {
    let r = Recovery::new();
    let signers = Vec::from_iter(
        &r.e,
        (0..MAX_SIGNERS).map(|i| {
            let mut key = [0u8; MAX_EXTERNAL_KEY_SIZE as usize];
            key[0] = i as u8;
            Signer::External(r.verifier.clone(), Bytes::from_slice(&r.e, &key))
        }),
    );
    let template = r.template(Map::new(&r.e));
    r.reach_quorum(&signers);
    r.at(EXECUTABLE_AFTER);

    let rule = r.complete(&template, &signers);
    // The whole transaction is this call, so it gets the full limit.
    assert_events_and_return_within(&r.e, rule, 100);
}
