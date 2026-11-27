extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    contract, contracterror, contractimpl, map, panic_with_error, symbol_short,
    testutils::Address as _,
    vec, Address, Env, IntoVal, Vec,
};

use crate::{
    policies::{
        simple_threshold::{self, SimpleThresholdAccountParams, SimpleThresholdError},
        spending_limit::{self, SpendingLimitAccountParams, SpendingLimitError},
        Policy,
    },
    smart_account::{get_context_rule, ContextRule, ContextRuleType, Signer},
    testutils::PolicyTestHarness,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
enum RequiredSignerError {
    MissingSigner = 3900,
}

#[contract]
struct RequiredSignerPolicy;

#[contractimpl]
impl Policy for RequiredSignerPolicy {
    type AccountParams = Signer;

    fn install(
        e: &Env,
        required_signer: Signer,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        smart_account.require_auth();
        assert!(context_rule.signers.contains(required_signer.clone()));
        e.storage().persistent().set(&(smart_account, context_rule.id), &required_signer);
    }

    fn enforce(
        e: &Env,
        _context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        smart_account.require_auth();
        let required_signer: Signer =
            e.storage().persistent().get(&(smart_account, context_rule.id)).unwrap();
        if !authenticated_signers.contains(required_signer) {
            panic_with_error!(e, RequiredSignerError::MissingSigner);
        }
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        smart_account.require_auth();
        e.storage().persistent().remove(&(smart_account, context_rule.id));
    }
}

#[test]
fn policy_receives_only_the_selected_signers() {
    let e = Env::default();
    let policy = e.register(RequiredSignerPolicy, ());
    let target = Address::generate(&e);
    let harness = PolicyTestHarness::new(
        &e,
        &ContextRuleType::CallContract(target.clone()),
        &[[1; 32], [2; 32], [3; 32]],
        |signers| map![&e, (policy.clone(), signers.get(2).unwrap().into_val(&e))],
    );
    let rule = e.as_contract(harness.account(), || get_context_rule(&e, harness.context_rule_id()));
    assert_eq!(rule.policies, vec![&e, policy]);

    let context = Context::Contract(ContractContext {
        contract: target,
        fn_name: symbol_short!("ping"),
        args: Vec::new(&e),
    });

    let missing_signer = Err(RequiredSignerError::MissingSigner.into());
    assert_eq!(harness.try_check_auth(context.clone(), &[]), missing_signer);
    assert_eq!(harness.try_check_auth(context.clone(), &[0, 1]), missing_signer);
    assert_eq!(harness.try_check_auth(context.clone(), &[2]), Ok(()));
    assert_eq!(harness.try_check_auth(context, &[0, 1, 2]), Ok(()));
}

#[contract]
struct SpendingLimitPolicy;

#[contractimpl]
impl Policy for SpendingLimitPolicy {
    type AccountParams = SpendingLimitAccountParams;

    fn install(e: &Env, params: Self::AccountParams, rule: ContextRule, account: Address) {
        spending_limit::install(e, &params, &rule, &account);
    }

    fn enforce(
        e: &Env,
        context: Context,
        signers: Vec<Signer>,
        rule: ContextRule,
        account: Address,
    ) {
        spending_limit::enforce(e, &context, &signers, &rule, &account);
    }

    fn uninstall(e: &Env, rule: ContextRule, account: Address) {
        spending_limit::uninstall(e, &rule, &account);
    }
}

#[contract]
struct SimpleThresholdPolicy;

#[contractimpl]
impl Policy for SimpleThresholdPolicy {
    type AccountParams = SimpleThresholdAccountParams;

    fn install(e: &Env, params: Self::AccountParams, rule: ContextRule, account: Address) {
        simple_threshold::install(e, &params, &rule, &account);
    }

    fn enforce(
        e: &Env,
        context: Context,
        signers: Vec<Signer>,
        rule: ContextRule,
        account: Address,
    ) {
        simple_threshold::enforce(e, &context, &signers, &rule, &account);
    }

    fn uninstall(e: &Env, rule: ContextRule, account: Address) {
        simple_threshold::uninstall(e, &rule, &account);
    }
}

#[test]
fn spending_limit_composes_with_signer_quorum() {
    let e = Env::default();
    let spending = e.register(SpendingLimitPolicy, ());
    let threshold = e.register(SimpleThresholdPolicy, ());
    let token = Address::generate(&e);
    let recipient = Address::generate(&e);
    let transfer = |account: &Address| {
        Context::Contract(ContractContext {
            contract: token.clone(),
            fn_name: symbol_short!("transfer"),
            args: vec![&e, account.into_val(&e), recipient.into_val(&e), 1i128.into_val(&e)],
        })
    };

    let rule_type = ContextRuleType::CallContract(token.clone());
    let seeds = [[1; 32], [2; 32], [3; 32]];
    let spending_params = SpendingLimitAccountParams { spending_limit: 10, period_ledgers: 100 };
    let mut policies = map![&e, (spending, spending_params.into_val(&e))];
    let spending_only = PolicyTestHarness::new(&e, &rule_type, &seeds, |_| policies.clone());
    assert_eq!(
        spending_only.try_check_auth(transfer(spending_only.account()), &[]),
        Err(SpendingLimitError::NotAllowed.into())
    );
    assert_eq!(spending_only.try_check_auth(transfer(spending_only.account()), &[2]), Ok(()));

    policies.set(threshold, SimpleThresholdAccountParams { threshold: 2 }.into_val(&e));
    let with_quorum = PolicyTestHarness::new(&e, &rule_type, &seeds, |_| policies);
    assert_eq!(
        with_quorum.try_check_auth(transfer(with_quorum.account()), &[2]),
        Err(SimpleThresholdError::NotAllowed.into())
    );
    assert_eq!(with_quorum.try_check_auth(transfer(with_quorum.account()), &[0, 2]), Ok(()));
}
