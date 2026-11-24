extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext, ContractExecutable, CreateContractHostFnContext},
    contract,
    testutils::{
        storage::{Instance as _, Persistent as _},
        Address as _, Events, Ledger,
    },
    vec,
    xdr::ContractEvent,
    Address, Bytes, BytesN, Env, Event, IntoVal, Map, String, Symbol, Val, Vec,
};
use stellar_test_utils::{assert_events_and_return_within, LIBRARY_HEADROOM_PERCENT};

use crate::{
    policies::social_recovery::*,
    smart_account::{
        ContextRule, ContextRuleType, Signer, MAX_EXTERNAL_KEY_SIZE, MAX_NAME_SIZE, MAX_POLICIES,
        MAX_SIGNERS,
    },
};

#[contract]
struct MockContract;

const START: u32 = 1_000_000;
const DELAY: u32 = 17_280;
const EXPIRY: u32 = 120_960;
const RULE_ID: u32 = 1;

// Running example: Bob opens at START, Carol reaches quorum at QUORUM.
const QUORUM: u32 = 1_010_000;
const EXECUTABLE_AFTER: u32 = 1_027_280;
const QUORUM_DEADLINE: u32 = 1_120_960;
const EXPIRES_AT: u32 = 1_148_240;

struct Fixture {
    e: Env,
    policy: Address,
    account: Address,
    rule: ContextRule,
    bob: Address,
    carol: Address,
    dave: Address,
    verifier: Address,
}

fn recovery_rule(e: &Env, id: u32, account: &Address, policy: &Address) -> ContextRule {
    ContextRule {
        id,
        context_type: ContextRuleType::CallContract(account.clone()),
        name: String::from_str(e, "recovery"),
        signers: Vec::new(e),
        signer_ids: Vec::new(e),
        policies: vec![e, policy.clone()],
        policy_ids: vec![e, 0],
        valid_until: None,
    }
}

fn completion_context(
    e: &Env,
    account: &Address,
    template: &RecoveryTemplate,
    signers: &Vec<Signer>,
) -> Context {
    Context::Contract(ContractContext {
        contract: account.clone(),
        fn_name: Symbol::new(e, "add_context_rule"),
        args: vec![
            e,
            template.context_type.into_val(e),
            template.name.into_val(e),
            None::<u32>.into_val(e),
            signers.into_val(e),
            template.policies.into_val(e),
        ],
    })
}

impl Fixture {
    fn new() -> Self {
        let e = Env::default();
        e.mock_all_auths();
        e.ledger().with_mut(|li| {
            li.sequence_number = START;
            li.min_persistent_entry_ttl = 1_000_000;
        });
        let policy = e.register(MockContract, ());
        let account = Address::generate(&e);
        let rule = recovery_rule(&e, RULE_ID, &account, &policy);
        let bob = Address::generate(&e);
        let carol = Address::generate(&e);
        let dave = Address::generate(&e);
        let verifier = Address::generate(&e);
        Fixture { e, policy, account, rule, bob, carol, dave, verifier }
    }

    fn installed() -> Self {
        let f = Self::new();
        f.install(&f.params());
        f
    }

    /// Installed with Bob's approval at START and Carol's at QUORUM, so the
    /// attempt on `signers(&[2])` is Ready with `executable_after =
    /// EXECUTABLE_AFTER`.
    fn ready() -> Self {
        let f = Self::installed();
        let signers = f.signers(&[2]);
        f.approve(&f.bob, &signers);
        f.at(QUORUM);
        f.approve(&f.carol, &signers);
        f
    }

    fn template(&self) -> RecoveryTemplate {
        RecoveryTemplate {
            context_type: ContextRuleType::Default,
            name: String::from_str(&self.e, "owner"),
            policies: Map::new(&self.e),
        }
    }

    fn params(&self) -> SocialRecoveryAccountParams {
        SocialRecoveryAccountParams {
            guardians: vec![&self.e, self.bob.clone(), self.carol.clone(), self.dave.clone()],
            threshold: 2,
            delay_ledgers: DELAY,
            expiry_ledgers: EXPIRY,
            template: self.template(),
        }
    }

    fn signer(&self, key: u8) -> Signer {
        Signer::External(self.verifier.clone(), Bytes::from_array(&self.e, &[key; 32]))
    }

    fn signers(&self, keys: &[u8]) -> Vec<Signer> {
        Vec::from_iter(&self.e, keys.iter().map(|key| self.signer(*key)))
    }

    fn hash(&self, signers: &Vec<Signer>) -> BytesN<32> {
        hash_signers(&self.e, signers)
    }

    fn at(&self, ledger: u32) {
        self.e.ledger().set_sequence_number(ledger);
    }

    fn install(&self, params: &SocialRecoveryAccountParams) {
        self.e.as_contract(&self.policy, || install(&self.e, params, &self.rule, &self.account));
    }

    fn install_on(&self, rule: &ContextRule, params: &SocialRecoveryAccountParams) {
        self.e.as_contract(&self.policy, || install(&self.e, params, rule, &self.account));
    }

    fn uninstall(&self) {
        self.e.as_contract(&self.policy, || uninstall(&self.e, &self.rule, &self.account));
    }

    fn approve(&self, guardian: &Address, signers: &Vec<Signer>) {
        let epoch = self.epoch();
        self.approve_at(guardian, epoch, signers);
    }

    fn approve_at(&self, guardian: &Address, epoch: u32, signers: &Vec<Signer>) {
        self.e.as_contract(&self.policy, || {
            approve(&self.e, guardian, epoch, signers, RULE_ID, &self.account)
        });
    }

    fn revoke(&self, guardian: &Address, signers: &Vec<Signer>) {
        let epoch = self.epoch();
        self.revoke_at(guardian, epoch, signers);
    }

    fn revoke_at(&self, guardian: &Address, epoch: u32, signers: &Vec<Signer>) {
        self.e.as_contract(&self.policy, || {
            revoke_approval(&self.e, guardian, epoch, signers, RULE_ID, &self.account)
        });
    }

    fn cancel(&self) {
        self.e.as_contract(&self.policy, || cancel(&self.e, RULE_ID, &self.account));
    }

    fn set_config(&self, params: &SocialRecoveryAccountParams) {
        self.e.as_contract(&self.policy, || set_config(&self.e, params, RULE_ID, &self.account));
    }

    fn enforce(&self, context: &Context) {
        self.e.as_contract(&self.policy, || {
            enforce(&self.e, context, &Vec::new(&self.e), &self.rule, &self.account)
        });
    }

    fn complete(&self, signers: &Vec<Signer>) {
        self.enforce(&self.completion(signers));
    }

    fn completion(&self, signers: &Vec<Signer>) -> Context {
        completion_context(&self.e, &self.account, &self.template(), signers)
    }

    /// Returns a completion context for `signers(&[2])` with argument `index`
    /// replaced by `value`.
    fn completion_with_arg(&self, index: u32, value: Val) -> Context {
        let Context::Contract(mut context) = self.completion(&self.signers(&[2])) else {
            unreachable!()
        };
        context.args.set(index, value);
        Context::Contract(context)
    }

    fn config(&self) -> SocialRecoveryAccountParams {
        self.e.as_contract(&self.policy, || get_config(&self.e, RULE_ID, &self.account))
    }

    fn epoch(&self) -> u32 {
        self.e.as_contract(&self.policy, || get_epoch(&self.e, RULE_ID, &self.account))
    }

    fn attempt_at(&self, epoch: u32, signers: &Vec<Signer>) -> Option<RecoveryAttempt> {
        let hash = self.hash(signers);
        self.e.as_contract(&self.policy, || {
            get_attempt(&self.e, epoch, &hash, RULE_ID, &self.account)
        })
    }

    fn attempt(&self, signers: &Vec<Signer>) -> Option<RecoveryAttempt> {
        let epoch = self.epoch();
        self.attempt_at(epoch, signers)
    }

    fn ttl(&self, key: &SocialRecoveryStorageKey) -> u32 {
        self.e.as_contract(&self.policy, || self.e.storage().persistent().get_ttl(key))
    }

    fn assert_events(&self, expected: &[ContractEvent]) {
        let all = self.e.events().all();
        let events = all.events();
        assert_eq!(events.len(), expected.len());
        for (event, expected) in events.iter().zip(expected) {
            assert_eq!(event, expected);
        }
    }

    fn approval_recorded(
        &self,
        epoch: u32,
        signers: &Vec<Signer>,
        guardian: &Address,
        approvals: u32,
        quorum_deadline: u32,
    ) -> ContractEvent {
        RecoveryApprovalRecorded {
            smart_account: self.account.clone(),
            context_rule_id: RULE_ID,
            epoch,
            signers_hash: self.hash(signers),
            signers: signers.clone(),
            guardian: guardian.clone(),
            approvals,
            quorum_deadline,
        }
        .to_xdr(&self.e, &self.policy)
    }

    fn quorum_reached(
        &self,
        epoch: u32,
        signers: &Vec<Signer>,
        executable_after: u32,
        expires_at: u32,
    ) -> ContractEvent {
        RecoveryQuorumReached {
            smart_account: self.account.clone(),
            context_rule_id: RULE_ID,
            epoch,
            signers_hash: self.hash(signers),
            executable_after,
            expires_at,
        }
        .to_xdr(&self.e, &self.policy)
    }

    fn approval_revoked(
        &self,
        epoch: u32,
        signers: &Vec<Signer>,
        guardian: &Address,
        approvals: u32,
        executable_after: Option<u32>,
    ) -> ContractEvent {
        RecoveryApprovalRevoked {
            smart_account: self.account.clone(),
            context_rule_id: RULE_ID,
            epoch,
            signers_hash: self.hash(signers),
            guardian: guardian.clone(),
            approvals,
            executable_after,
        }
        .to_xdr(&self.e, &self.policy)
    }
}

fn attempt(e: &Env, opened_at: u32, approvals: &[&Address], ea: Option<u32>) -> RecoveryAttempt {
    RecoveryAttempt {
        opened_at,
        approvals: Vec::from_iter(e, approvals.iter().map(|a| (*a).clone())),
        executable_after: ea,
    }
}

// ################## INSTALL AND CONFIGURATION ##################

#[test]
fn install_works() {
    let f = Fixture::new();
    let params = f.params();
    f.install(&params);
    f.assert_events(&[RecoveryInstalled {
        smart_account: f.account.clone(),
        context_rule_id: RULE_ID,
        config: params.clone(),
        epoch: 1,
    }
    .to_xdr(&f.e, &f.policy)]);
    assert_eq!(f.config(), params);
    assert_eq!(f.epoch(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_without_guardians_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = Vec::new(&f.e);
    params.threshold = 1;
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_too_many_guardians_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = Vec::from_iter(&f.e, (0..=MAX_GUARDIANS).map(|_| Address::generate(&f.e)));
    f.install(&params);
}

#[test]
fn install_max_guardians_works() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = Vec::from_iter(&f.e, (0..MAX_GUARDIANS).map(|_| Address::generate(&f.e)));
    params.threshold = MAX_GUARDIANS;
    f.install(&params);
    assert_eq!(f.config(), params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_duplicate_guardian_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = vec![&f.e, f.bob.clone(), f.carol.clone(), f.bob.clone()];
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_account_as_guardian_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = vec![&f.e, f.bob.clone(), f.carol.clone(), f.account.clone()];
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_policy_as_guardian_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = vec![&f.e, f.bob.clone(), f.carol.clone(), f.policy.clone()];
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_zero_threshold_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 0;
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_threshold_above_guardians_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 4;
    f.install(&params);
}

#[test]
fn install_threshold_equal_to_guardians_works() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 3;
    f.install(&params);
    assert_eq!(f.config().threshold, 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_zero_delay_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.delay_ledgers = 0;
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_zero_expiry_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.expiry_ledgers = 0;
    f.install(&params);
}

#[test]
fn install_max_delay_works() {
    let f = Fixture::new();
    let mut params = f.params();
    params.delay_ledgers = u32::MAX - START - 2 * EXPIRY;
    f.install(&params);
    assert_eq!(f.config(), params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_delay_past_ledger_headroom_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.delay_ledgers = u32::MAX - START - 2 * EXPIRY + 1;
    f.install(&params);
}

#[test]
fn install_max_expiry_works() {
    let f = Fixture::new();
    let mut params = f.params();
    params.expiry_ledgers = (u32::MAX - START - DELAY) / 2;
    f.install(&params);
    assert_eq!(f.config(), params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_expiry_past_ledger_headroom_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    // `START + DELAY + expiry` fits, `START + DELAY + 2 * expiry` does not.
    params.expiry_ledgers = (u32::MAX - START - DELAY) / 2 + 1;
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_for_other_contract_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.context_type = ContextRuleType::CallContract(Address::generate(&f.e));
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_create_contract_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.context_type =
        ContextRuleType::CreateContract(BytesN::from_array(&f.e, &[0; 32]));
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_create_contract_external_ref_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.context_type = ContextRuleType::CreateContractExternalRef(
        Address::generate(&f.e),
        String::from_str(&f.e, "tag"),
    );
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_name_too_long_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.name = String::from_str(&f.e, "abcdefghijklmnopqrstu");
    assert_eq!(params.template.name.len(), MAX_NAME_SIZE + 1);
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_too_many_policies_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    for _ in 0..=MAX_POLICIES {
        params.template.policies.set(Address::generate(&f.e), 1u32.into_val(&f.e));
    }
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_with_this_policy_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.policies.set(f.policy.clone(), 1u32.into_val(&f.e));
    f.install(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_template_with_account_policy_panics() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.policies.set(f.account.clone(), 1u32.into_val(&f.e));
    f.install(&params);
}

#[test]
fn install_template_at_limits_works() {
    let f = Fixture::new();
    let mut params = f.params();
    params.template.context_type = ContextRuleType::CallContract(f.account.clone());
    params.template.name = String::from_str(&f.e, "abcdefghijklmnopqrst");
    assert_eq!(params.template.name.len(), MAX_NAME_SIZE);
    for _ in 0..MAX_POLICIES {
        params.template.policies.set(Address::generate(&f.e), 1u32.into_val(&f.e));
    }
    f.install(&params);
    assert_eq!(f.config(), params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_on_default_rule_panics() {
    let f = Fixture::new();
    let mut rule = f.rule.clone();
    rule.context_type = ContextRuleType::Default;
    f.install_on(&rule, &f.params());
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn install_on_other_contract_rule_panics() {
    let f = Fixture::new();
    let mut rule = f.rule.clone();
    rule.context_type = ContextRuleType::CallContract(Address::generate(&f.e));
    f.install_on(&rule, &f.params());
}

#[test]
#[should_panic(expected = "Error(Contract, #3231)")] // SocialRecoveryError::AlreadyInstalled
fn install_twice_panics() {
    let f = Fixture::installed();
    f.install(&f.params());
}

#[test]
fn set_config_works() {
    let f = Fixture::installed();
    let mut params = f.params();
    params.guardians = vec![&f.e, f.carol.clone(), f.dave.clone()];
    params.threshold = 1;
    f.set_config(&params);
    f.assert_events(&[RecoveryConfigUpdated {
        smart_account: f.account.clone(),
        context_rule_id: RULE_ID,
        config: params.clone(),
        epoch: 2,
    }
    .to_xdr(&f.e, &f.policy)]);
    assert_eq!(f.config(), params);
    assert_eq!(f.epoch(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn set_config_invalid_config_panics() {
    let f = Fixture::installed();
    let mut params = f.params();
    params.threshold = 0;
    f.set_config(&params);
}

#[test]
#[should_panic(expected = "Error(Contract, #3232)")] // SocialRecoveryError::InvalidConfig
fn set_config_invalid_template_panics() {
    let f = Fixture::installed();
    let mut params = f.params();
    params.template.policies.set(f.policy.clone(), 1u32.into_val(&f.e));
    f.set_config(&params);
}

#[test]
fn uninstall_works() {
    let f = Fixture::installed();
    f.uninstall();
    f.assert_events(&[RecoveryUninstalled {
        smart_account: f.account.clone(),
        context_rule_id: RULE_ID,
    }
    .to_xdr(&f.e, &f.policy)]);
    f.e.as_contract(&f.policy, || {
        let storage = f.e.storage().persistent();
        assert!(!storage.has(&SocialRecoveryStorageKey::Config(f.account.clone(), RULE_ID)));
        assert!(storage.has(&SocialRecoveryStorageKey::Epoch(f.account.clone(), RULE_ID)));
    });
    assert_eq!(f.epoch(), 1);
}

// ################## NOT INSTALLED ##################

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn uninstall_not_installed_panics() {
    Fixture::new().uninstall();
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn set_config_not_installed_panics() {
    let f = Fixture::new();
    f.set_config(&f.params());
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn cancel_not_installed_panics() {
    Fixture::new().cancel();
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn approve_not_installed_panics() {
    let f = Fixture::new();
    f.approve_at(&f.bob, 0, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn revoke_not_installed_panics() {
    let f = Fixture::new();
    f.revoke_at(&f.bob, 0, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn enforce_not_installed_panics() {
    let f = Fixture::new();
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn get_config_not_installed_panics() {
    Fixture::new().config();
}

#[test]
#[should_panic(expected = "Error(Contract, #3230)")] // SocialRecoveryError::NotInstalled
fn approve_after_uninstall_panics() {
    let f = Fixture::installed();
    f.uninstall();
    f.approve_at(&f.bob, 1, &f.signers(&[2]));
}

// ################## GUARDIANS AND EPOCHS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3234)")] // SocialRecoveryError::NotGuardian
fn approve_non_guardian_panics() {
    let f = Fixture::installed();
    f.approve(&Address::generate(&f.e), &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3234)")] // SocialRecoveryError::NotGuardian
fn revoke_non_guardian_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.revoke(&Address::generate(&f.e), &signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3235)")] // SocialRecoveryError::StaleEpoch
fn approve_old_epoch_after_cancel_panics() {
    let f = Fixture::installed();
    f.cancel();
    f.approve_at(&f.bob, 1, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3235)")] // SocialRecoveryError::StaleEpoch
fn approve_future_epoch_panics() {
    let f = Fixture::installed();
    f.approve_at(&f.bob, 2, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3235)")] // SocialRecoveryError::StaleEpoch
fn revoke_old_epoch_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.cancel();
    f.revoke_at(&f.bob, 1, &signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3235)")] // SocialRecoveryError::StaleEpoch
fn approve_signed_before_reinstall_panics() {
    let f = Fixture::installed();
    f.uninstall();
    f.install(&f.params());
    assert_eq!(f.epoch(), 2);
    f.approve_at(&f.bob, 1, &f.signers(&[2]));
}

#[test]
fn epoch_increments_on_install_cancel_set_config_and_completion() {
    let f = Fixture::new();
    assert_eq!(f.epoch(), 0);
    f.install(&f.params());
    assert_eq!(f.epoch(), 1);
    f.cancel();
    assert_eq!(f.epoch(), 2);
    f.set_config(&f.params());
    assert_eq!(f.epoch(), 3);
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.approve(&f.carol, &signers);
    f.at(START + DELAY);
    f.complete(&signers);
    assert_eq!(f.epoch(), 4);
}

#[test]
fn reinstall_continues_epoch() {
    let f = Fixture::installed();
    f.uninstall();
    assert_eq!(f.epoch(), 1);
    f.install(&f.params());
    f.assert_events(&[RecoveryInstalled {
        smart_account: f.account.clone(),
        context_rule_id: RULE_ID,
        config: f.params(),
        epoch: 2,
    }
    .to_xdr(&f.e, &f.policy)]);
    assert_eq!(f.epoch(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3239)")] // SocialRecoveryError::MathOverflow
fn cancel_at_max_epoch_panics() {
    let f = Fixture::installed();
    f.e.as_contract(&f.policy, || {
        f.e.storage()
            .persistent()
            .set(&SocialRecoveryStorageKey::Epoch(f.account.clone(), RULE_ID), &u32::MAX);
    });
    f.cancel();
}

#[test]
fn cancel_without_attempt_bumps_epoch() {
    let f = Fixture::installed();
    f.cancel();
    f.assert_events(&[RecoveryCancelled {
        smart_account: f.account.clone(),
        context_rule_id: RULE_ID,
        epoch: 2,
    }
    .to_xdr(&f.e, &f.policy)]);
    assert_eq!(f.epoch(), 2);
}

// ################## SIGNER SETS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3233)")] // SocialRecoveryError::InvalidSigners
fn approve_empty_signers_panics() {
    let f = Fixture::installed();
    f.approve(&f.bob, &Vec::new(&f.e));
}

#[test]
#[should_panic(expected = "Error(Contract, #3233)")] // SocialRecoveryError::InvalidSigners
fn approve_too_many_signers_panics() {
    let f = Fixture::installed();
    let keys: std::vec::Vec<u8> = (0..=MAX_SIGNERS as u8).collect();
    f.approve(&f.bob, &f.signers(&keys));
}

#[test]
#[should_panic(expected = "Error(Contract, #3233)")] // SocialRecoveryError::InvalidSigners
fn approve_oversized_external_key_panics() {
    let f = Fixture::installed();
    let key = Bytes::from_slice(&f.e, &[7; MAX_EXTERNAL_KEY_SIZE as usize + 1]);
    f.approve(&f.bob, &vec![&f.e, Signer::External(f.verifier.clone(), key)]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3233)")] // SocialRecoveryError::InvalidSigners
fn approve_self_delegated_signer_panics() {
    let f = Fixture::installed();
    f.approve(&f.bob, &vec![&f.e, f.signer(2), Signer::Delegated(f.account.clone())]);
}

#[test]
#[should_panic(expected = "Error(Contract, #3233)")] // SocialRecoveryError::InvalidSigners
fn approve_duplicate_signer_panics() {
    let f = Fixture::installed();
    f.approve(&f.bob, &f.signers(&[2, 3, 2]));
}

#[test]
fn approve_signers_at_limits_works() {
    let f = Fixture::installed();
    let mut signers = Vec::new(&f.e);
    for i in 0..MAX_SIGNERS - 1 {
        let mut key = [0u8; MAX_EXTERNAL_KEY_SIZE as usize];
        key[0] = i as u8;
        signers.push_back(Signer::External(f.verifier.clone(), Bytes::from_slice(&f.e, &key)));
    }
    signers.push_back(Signer::Delegated(Address::generate(&f.e)));
    assert_eq!(signers.len(), MAX_SIGNERS);
    f.approve(&f.bob, &signers);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, START, &[&f.bob], None)));
}

#[test]
fn hash_signers_is_order_sensitive_sha256_of_xdr() {
    let f = Fixture::new();
    let forward = f.signers(&[2, 3]);
    let reversed = f.signers(&[3, 2]);
    let xdr = soroban_sdk::xdr::ToXdr::to_xdr(forward.clone(), &f.e);
    assert_eq!(f.hash(&forward), f.e.crypto().sha256(&xdr).to_bytes());
    assert_ne!(f.hash(&forward), f.hash(&reversed));
}

// ################## ATTEMPT STATE MACHINE ##################

#[test]
fn approve_opens_attempt() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.assert_events(&[f.approval_recorded(1, &signers, &f.bob, 1, QUORUM_DEADLINE)]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, START, &[&f.bob], None)));
}

#[test]
fn approve_reaching_threshold_sets_executable_after() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.at(QUORUM);
    f.approve(&f.carol, &signers);
    f.assert_events(&[
        f.approval_recorded(1, &signers, &f.carol, 2, QUORUM_DEADLINE),
        f.quorum_reached(1, &signers, EXECUTABLE_AFTER, EXPIRES_AT),
    ]);
    assert_eq!(
        f.attempt(&signers),
        Some(attempt(&f.e, START, &[&f.bob, &f.carol], Some(EXECUTABLE_AFTER)))
    );
}

#[test]
fn approve_with_threshold_one_opens_ready_attempt() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 1;
    f.install(&params);
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.assert_events(&[
        f.approval_recorded(1, &signers, &f.bob, 1, QUORUM_DEADLINE),
        f.quorum_reached(1, &signers, START + DELAY, START + DELAY + EXPIRY),
    ]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, START, &[&f.bob], Some(START + DELAY))));
}

#[test]
fn approval_before_quorum_deadline_reaches_quorum() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.at(QUORUM_DEADLINE - 1);
    f.approve(&f.carol, &signers);
    assert_eq!(
        f.attempt(&signers),
        Some(attempt(&f.e, START, &[&f.bob, &f.carol], Some(QUORUM_DEADLINE - 1 + DELAY)))
    );
}

#[test]
fn approve_expired_collecting_attempt_opens_fresh() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.at(QUORUM_DEADLINE);
    f.approve(&f.carol, &signers);
    f.assert_events(&[f.approval_recorded(1, &signers, &f.carol, 1, QUORUM_DEADLINE + EXPIRY)]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, QUORUM_DEADLINE, &[&f.carol], None)));
}

#[test]
fn approve_expired_ready_attempt_opens_fresh() {
    let f = Fixture::ready();
    let signers = f.signers(&[2]);
    f.at(EXPIRES_AT - 1);
    assert_eq!(
        f.attempt(&signers).unwrap().executable_after,
        Some(EXECUTABLE_AFTER),
        "still Ready one ledger before expiry"
    );
    f.at(EXPIRES_AT);
    f.approve(&f.carol, &signers);
    f.assert_events(&[f.approval_recorded(1, &signers, &f.carol, 1, EXPIRES_AT + EXPIRY)]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, EXPIRES_AT, &[&f.carol], None)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3236)")] // SocialRecoveryError::AlreadyApproved
fn approve_twice_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.approve(&f.bob, &signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3236)")] // SocialRecoveryError::AlreadyApproved
fn approve_twice_on_ready_attempt_panics() {
    let f = Fixture::ready();
    f.approve(&f.bob, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3237)")] // SocialRecoveryError::ApprovalNotFound
fn revoke_without_approval_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.revoke(&f.carol, &signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3237)")] // SocialRecoveryError::ApprovalNotFound
fn revoke_absent_attempt_panics() {
    let f = Fixture::installed();
    f.revoke(&f.bob, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3237)")] // SocialRecoveryError::ApprovalNotFound
fn revoke_expired_attempt_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.at(QUORUM_DEADLINE);
    f.revoke(&f.bob, &signers);
}

#[test]
fn revoke_keeps_collecting_attempt() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 3;
    f.install(&params);
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.approve(&f.carol, &signers);
    f.revoke(&f.carol, &signers);
    f.assert_events(&[f.approval_revoked(1, &signers, &f.carol, 1, None)]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, START, &[&f.bob], None)));
}

#[test]
fn revoke_last_approval_removes_attempt() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.revoke(&f.bob, &signers);
    f.assert_events(&[f.approval_revoked(1, &signers, &f.bob, 0, None)]);
    assert_eq!(f.attempt(&signers), None);
}

#[test]
fn revoke_only_approval_of_ready_attempt_removes_it() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 1;
    f.install(&params);
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.revoke(&f.bob, &signers);
    f.assert_events(&[f.approval_revoked(1, &signers, &f.bob, 0, None)]);
    assert_eq!(f.attempt(&signers), None);
}

#[test]
fn revoke_below_threshold_after_quorum_deadline_expires_attempt() {
    let f = Fixture::ready();
    let signers = f.signers(&[2]);
    f.at(QUORUM_DEADLINE);
    f.revoke(&f.carol, &signers);
    f.assert_events(&[f.approval_revoked(1, &signers, &f.carol, 1, None)]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, START, &[&f.bob], None)));

    // The remaining attempt is Expired, so the next approval opens a fresh one.
    f.approve(&f.dave, &signers);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, QUORUM_DEADLINE, &[&f.dave], None)));
}

#[test]
fn executable_after_moves_only_on_new_quorum() {
    let f = Fixture::ready();
    let signers = f.signers(&[2]);

    // An approval after quorum is recorded and leaves `executable_after`.
    f.at(1_020_000);
    f.approve(&f.dave, &signers);
    f.assert_events(&[f.approval_recorded(1, &signers, &f.dave, 3, QUORUM_DEADLINE)]);
    assert_eq!(
        f.attempt(&signers),
        Some(attempt(&f.e, START, &[&f.bob, &f.carol, &f.dave], Some(EXECUTABLE_AFTER)))
    );

    // A revoke that keeps the threshold leaves it.
    f.at(1_021_000);
    f.revoke(&f.carol, &signers);
    f.assert_events(&[f.approval_revoked(1, &signers, &f.carol, 2, Some(EXECUTABLE_AFTER))]);
    assert_eq!(
        f.attempt(&signers),
        Some(attempt(&f.e, START, &[&f.bob, &f.dave], Some(EXECUTABLE_AFTER)))
    );

    // A revoke below the threshold clears it.
    f.at(1_022_000);
    f.revoke(&f.bob, &signers);
    f.assert_events(&[f.approval_revoked(1, &signers, &f.bob, 1, None)]);
    assert_eq!(f.attempt(&signers), Some(attempt(&f.e, START, &[&f.dave], None)));

    // The next quorum starts a new delay.
    f.at(1_023_000);
    f.approve(&f.bob, &signers);
    f.assert_events(&[
        f.approval_recorded(1, &signers, &f.bob, 2, QUORUM_DEADLINE),
        f.quorum_reached(1, &signers, 1_040_280, 1_161_240),
    ]);
    assert_eq!(
        f.attempt(&signers),
        Some(attempt(&f.e, START, &[&f.dave, &f.bob], Some(1_040_280)))
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3239)")] // SocialRecoveryError::MathOverflow
fn approve_quorum_deadline_overflow_panics() {
    let f = Fixture::new();
    // Keep every entry live from START to the last ledgers before u32::MAX.
    f.e.ledger().with_mut(|li| {
        li.min_persistent_entry_ttl = u32::MAX - START;
        li.max_entry_ttl = u32::MAX - START;
    });
    f.e.as_contract(&f.policy, || {
        f.e.storage().instance().extend_ttl(u32::MAX - START - 1, u32::MAX - START - 1)
    });
    let mut params = f.params();
    params.expiry_ledgers = 1_000_000_000;
    params.delay_ledgers = 1;
    f.install(&params);

    // Later in the network's life `opened_at + expiry_ledgers` no longer fits.
    let late = u32::MAX - 500_000_000;
    f.e.ledger().with_mut(|li| {
        li.sequence_number = late;
        li.min_persistent_entry_ttl = 4096;
        li.max_entry_ttl = 1_000_000;
    });
    f.approve(&f.bob, &f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3239)")] // SocialRecoveryError::MathOverflow
fn approve_executable_after_overflow_panics() {
    let f = Fixture::new();
    f.e.ledger().with_mut(|li| {
        li.min_persistent_entry_ttl = u32::MAX - START;
        li.max_entry_ttl = u32::MAX - START;
    });
    f.e.as_contract(&f.policy, || {
        f.e.storage().instance().extend_ttl(u32::MAX - START - 1, u32::MAX - START - 1)
    });
    let mut params = f.params();
    params.threshold = 1;
    params.delay_ledgers = 2_000_000_000;
    params.expiry_ledgers = 1;
    f.install(&params);

    // `opened_at + expiry_ledgers` fits, `L + delay_ledgers` does not.
    f.e.ledger().with_mut(|li| {
        li.sequence_number = 3_000_000_000;
        li.min_persistent_entry_ttl = 4096;
        li.max_entry_ttl = 1_000_000;
    });
    f.approve(&f.bob, &f.signers(&[2]));
}

// ################## COMPLETION ##################

#[test]
fn enforce_completion_works() {
    let f = Fixture::ready();
    let signers = f.signers(&[2]);
    f.at(EXECUTABLE_AFTER);
    f.complete(&signers);
    f.assert_events(&[RecoveryAuthorized {
        smart_account: f.account.clone(),
        context_rule_id: RULE_ID,
        epoch: 1,
        signers_hash: f.hash(&signers),
    }
    .to_xdr(&f.e, &f.policy)]);
    assert_eq!(f.attempt_at(1, &signers), None);
    assert_eq!(f.epoch(), 2);
}

#[test]
fn enforce_at_last_executable_ledger_works() {
    let f = Fixture::ready();
    f.at(EXPIRES_AT - 1);
    f.complete(&f.signers(&[2]));
    assert_eq!(f.epoch(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_before_executable_after_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER - 1);
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_at_expiry_panics() {
    let f = Fixture::ready();
    f.at(EXPIRES_AT);
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_collecting_attempt_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);
    f.at(QUORUM_DEADLINE - 1);
    f.complete(&signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_unapproved_signers_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.complete(&f.signers(&[3]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_reordered_signers_panics() {
    let f = Fixture::installed();
    let signers = f.signers(&[2, 3]);
    f.approve(&f.bob, &signers);
    f.approve(&f.carol, &signers);
    f.at(START + DELAY);
    f.complete(&f.signers(&[3, 2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_other_contract_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    let Context::Contract(mut context) = f.completion(&f.signers(&[2])) else { unreachable!() };
    context.contract = Address::generate(&f.e);
    f.enforce(&Context::Contract(context));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_other_fn_name_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    let Context::Contract(mut context) = f.completion(&f.signers(&[2])) else { unreachable!() };
    context.fn_name = Symbol::new(&f.e, "execute");
    f.enforce(&Context::Contract(context));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_missing_arg_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    let Context::Contract(mut context) = f.completion(&f.signers(&[2])) else { unreachable!() };
    context.args.pop_back();
    f.enforce(&Context::Contract(context));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_extra_arg_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    let Context::Contract(mut context) = f.completion(&f.signers(&[2])) else { unreachable!() };
    context.args.push_back(1u32.into_val(&f.e));
    f.enforce(&Context::Contract(context));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_wrong_context_type_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    let context_type = ContextRuleType::CallContract(f.account.clone());
    f.enforce(&f.completion_with_arg(0, context_type.into_val(&f.e)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_wrong_name_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.enforce(&f.completion_with_arg(1, String::from_str(&f.e, "attacker").into_val(&f.e)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_with_valid_until_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.enforce(&f.completion_with_arg(2, Some(EXPIRES_AT).into_val(&f.e)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_undecodable_signers_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.enforce(&f.completion_with_arg(3, 7u32.into_val(&f.e)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_wrong_policies_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    let mut policies = Map::<Address, Val>::new(&f.e);
    policies.set(Address::generate(&f.e), 1u32.into_val(&f.e));
    f.enforce(&f.completion_with_arg(4, policies.into_val(&f.e)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_create_contract_context_panics() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.enforce(&Context::CreateContractHostFn(CreateContractHostFnContext {
        executable: ContractExecutable::Wasm(BytesN::from_array(&f.e, &[1; 32])),
        salt: BytesN::from_array(&f.e, &[2; 32]),
    }));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn enforce_replay_panics() {
    let f = Fixture::ready();
    let signers = f.signers(&[2]);
    f.at(EXECUTABLE_AFTER);
    f.complete(&signers);
    f.complete(&signers);
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn completion_retires_sibling_ready_attempt() {
    let f = Fixture::installed();
    let first = f.signers(&[2]);
    let second = f.signers(&[3]);
    f.approve(&f.bob, &first);
    f.approve(&f.carol, &first);
    f.approve(&f.carol, &second);
    f.approve(&f.dave, &second);
    f.at(START + DELAY);
    f.complete(&first);
    f.complete(&second);
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn cancel_before_executable_after_blocks_completion() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER - 1);
    f.cancel();
    f.at(EXECUTABLE_AFTER);
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn cancel_after_executable_after_blocks_completion() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER + 5);
    f.cancel();
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn cancel_in_completion_ledger_blocks_completion() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.cancel();
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn set_config_after_quorum_blocks_completion() {
    let f = Fixture::ready();
    f.set_config(&f.params());
    f.at(EXECUTABLE_AFTER);
    f.complete(&f.signers(&[2]));
}

#[test]
#[should_panic(expected = "Error(Contract, #3238)")] // SocialRecoveryError::NotAllowed
fn dead_epoch_attempt_is_never_completed() {
    let f = Fixture::ready();
    f.cancel();
    let signers = f.signers(&[2]);
    assert!(f.attempt_at(1, &signers).is_some());
    f.at(EXECUTABLE_AFTER);
    f.complete(&signers);
}

// ################## TENANTS ##################

#[test]
fn cancel_leaves_other_tenants_untouched() {
    let f = Fixture::installed();
    let other_account = Address::generate(&f.e);
    let second_rule = recovery_rule(&f.e, 2, &f.account, &f.policy);
    let other_rule = recovery_rule(&f.e, RULE_ID, &other_account, &f.policy);
    f.install_on(&second_rule, &f.params());
    f.e.as_contract(&f.policy, || install(&f.e, &f.params(), &other_rule, &other_account));
    let signers = f.signers(&[2]);
    f.e.as_contract(&f.policy, || approve(&f.e, &f.bob, 1, &signers, 2, &f.account));
    f.e.as_contract(&f.policy, || approve(&f.e, &f.bob, 1, &signers, RULE_ID, &other_account));

    f.cancel();

    let hash = f.hash(&signers);
    f.e.as_contract(&f.policy, || {
        assert_eq!(get_epoch(&f.e, RULE_ID, &f.account), 2);
        assert_eq!(get_epoch(&f.e, 2, &f.account), 1);
        assert_eq!(get_epoch(&f.e, RULE_ID, &other_account), 1);
        assert!(get_attempt(&f.e, 1, &hash, 2, &f.account).is_some());
        assert!(get_attempt(&f.e, 1, &hash, RULE_ID, &other_account).is_some());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3234)")] // SocialRecoveryError::NotGuardian
fn guardian_of_other_tenant_panics() {
    let f = Fixture::installed();
    let other_account = Address::generate(&f.e);
    let erin = Address::generate(&f.e);
    let other_rule = recovery_rule(&f.e, RULE_ID, &other_account, &f.policy);
    let mut other_params = f.params();
    other_params.guardians = vec![&f.e, erin.clone()];
    other_params.threshold = 1;
    f.e.as_contract(&f.policy, || install(&f.e, &other_params, &other_rule, &other_account));
    f.approve(&erin, &f.signers(&[2]));
}

// ################## VIEWS AND STORAGE ##################

#[test]
fn get_epoch_is_zero_when_never_installed() {
    assert_eq!(Fixture::new().epoch(), 0);
}

#[test]
fn get_attempt_is_none_when_absent() {
    let f = Fixture::installed();
    assert_eq!(f.attempt(&f.signers(&[2])), None);
}

#[test]
fn reads_extend_ttl() {
    let f = Fixture::new();
    f.e.ledger().set_min_persistent_entry_ttl(500);
    f.install(&f.params());
    let signers = f.signers(&[2]);
    f.approve(&f.bob, &signers);

    let config = SocialRecoveryStorageKey::Config(f.account.clone(), RULE_ID);
    let epoch = SocialRecoveryStorageKey::Epoch(f.account.clone(), RULE_ID);
    let attempt =
        SocialRecoveryStorageKey::Attempt(f.account.clone(), RULE_ID, 1, f.hash(&signers));
    // `approve` read the config and epoch; the attempt was only written.
    assert_eq!(f.ttl(&config), SOCIAL_RECOVERY_EXTEND_AMOUNT);
    assert_eq!(f.ttl(&epoch), SOCIAL_RECOVERY_EXTEND_AMOUNT);
    assert_eq!(f.ttl(&attempt), 499);

    f.e.as_contract(&f.policy, || get_attempt(&f.e, 1, &f.hash(&signers), RULE_ID, &f.account));
    assert_eq!(f.ttl(&attempt), SOCIAL_RECOVERY_EXTEND_AMOUNT);
}

#[test]
fn writes_do_not_extend_ttl() {
    let f = Fixture::new();
    f.e.ledger().set_min_persistent_entry_ttl(500);
    f.install(&f.params());
    let config = SocialRecoveryStorageKey::Config(f.account.clone(), RULE_ID);
    let epoch = SocialRecoveryStorageKey::Epoch(f.account.clone(), RULE_ID);
    assert_eq!(f.ttl(&config), 499);
    assert_eq!(f.ttl(&epoch), 499);

    // `set_config` rewrites the config without reading it, and reads the epoch
    // before rewriting it.
    f.at(START + 100);
    f.set_config(&f.params());
    assert_eq!(f.ttl(&config), 399);
    assert_eq!(f.ttl(&epoch), SOCIAL_RECOVERY_EXTEND_AMOUNT);
}

#[test]
fn policy_uses_no_instance_storage() {
    let f = Fixture::ready();
    f.at(EXECUTABLE_AFTER);
    f.complete(&f.signers(&[2]));
    f.cancel();
    f.uninstall();
    f.e.as_contract(&f.policy, || assert!(f.e.storage().instance().all().is_empty()));
}

// ################## EVENT SIZES ##################

#[test]
fn max_signer_approval_events_fit() {
    let f = Fixture::new();
    let mut params = f.params();
    params.threshold = 1;
    f.install(&params);
    let signers = Vec::from_iter(
        &f.e,
        (0..MAX_SIGNERS).map(|i| {
            let mut key = [0u8; MAX_EXTERNAL_KEY_SIZE as usize];
            key[0] = i as u8;
            Signer::External(Address::generate(&f.e), Bytes::from_slice(&f.e, &key))
        }),
    );
    f.approve(&f.bob, &signers);
    assert_eq!(f.e.events().all().events().len(), 2);
    assert_events_and_return_within(&f.e, (), LIBRARY_HEADROOM_PERCENT);
}

#[test]
fn max_guardian_install_events_fit() {
    let f = Fixture::new();
    let mut params = f.params();
    params.guardians = Vec::from_iter(&f.e, (0..MAX_GUARDIANS).map(|_| Address::generate(&f.e)));
    params.template.name = String::from_str(&f.e, "abcdefghijklmnopqrst");
    for _ in 0..MAX_POLICIES {
        params.template.policies.set(Address::generate(&f.e), 1u32.into_val(&f.e));
    }
    f.install(&params);
    assert_eq!(f.e.events().all().events().len(), 1);
    assert_events_and_return_within(&f.e, (), LIBRARY_HEADROOM_PERCENT);
}
