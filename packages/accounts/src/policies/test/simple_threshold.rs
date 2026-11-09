extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    contract, symbol_short,
    testutils::{Address as _, Events},
    xdr::{Limits, WriteXdr},
    Address, Bytes, Env, Event, IntoVal, String, Vec,
};

use super::ENFORCED_EVENT_SIZE_CEILING_BYTES;
use crate::{
    policies::{simple_threshold::*, EnforcedContext},
    smart_account::{ContextRule, ContextRuleType, MAX_EXTERNAL_KEY_SIZE, MAX_SIGNERS},
};

#[contract]
struct MockContract;

fn create_test_signers(e: &Env) -> (Address, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);
    let addr3 = Address::generate(e);

    (addr1, addr2, addr3)
}

fn create_test_context_rule(e: &Env) -> ContextRule {
    let (addr1, addr2, addr3) = create_test_signers(e);
    let mut signers = Vec::new(e);
    signers.push_back(Signer::Delegated(addr1));
    signers.push_back(Signer::Delegated(addr2));
    signers.push_back(Signer::Delegated(addr3));
    let policies = Vec::new(e);
    ContextRule {
        id: 1,
        context_type: ContextRuleType::Default,
        name: String::from_str(e, "test_rule"),
        signers,
        signer_ids: Vec::from_array(e, [1, 2, 3]),
        policies,
        policy_ids: Vec::new(e),
        valid_until: None,
    }
}

fn create_max_external_signer_context_rule(e: &Env) -> ContextRule {
    let mut signers = Vec::new(e);
    let mut signer_ids = Vec::new(e);
    for index in 0..MAX_SIGNERS {
        let key = std::vec![index as u8; MAX_EXTERNAL_KEY_SIZE as usize];
        signers.push_back(Signer::External(Address::generate(e), Bytes::from_slice(e, &key)));
        signer_ids.push_back(index + 1);
    }

    ContextRule {
        id: 2,
        context_type: ContextRuleType::Default,
        name: String::from_str(e, "max_external_signers"),
        signers,
        signer_ids,
        policies: Vec::new(e),
        policy_ids: Vec::new(e),
        valid_until: None,
    }
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 2);

        // Verify install event was emitted
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3203)")]
fn install_already_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let (_, _, _) = create_test_signers(&e);
    let params = SimpleThresholdAccountParams { threshold: 2 };
    let context_rule = create_test_context_rule(&e);

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3201)")]
fn install_zero_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 0 }; // Invalid
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3200)")]
fn smart_account_get_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        get_threshold(&e, context_rule.id, &smart_account);
    });
}

#[test]
fn enforce_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_test_context_rule(&e);
    let authenticated_signers = Vec::from_array(
        &e,
        [context_rule.signers.get_unchecked(0), context_rule.signers.get_unchecked(1)],
    );

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 2 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = Context::Contract(ContractContext {
            contract: Address::generate(&e),
            fn_name: symbol_short!("test"),
            args: ().into_val(&e),
        });

        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn enforce_with_missing_aligned_signer_id_does_not_panic() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let mut context_rule = create_test_context_rule(&e);
    context_rule.signer_ids = Vec::new(&e);
    let authenticated_signers = Vec::from_array(&e, [context_rule.signers.get_unchecked(0)]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 1 };
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let contract = Address::generate(&e);
        let fn_name = symbol_short!("test");
        let context = Context::Contract(ContractContext {
            contract: contract.clone(),
            fn_name: fn_name.clone(),
            args: ().into_val(&e),
        });

        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        let events = e.events().all();
        assert_eq!(events.events().len(), 1);
        assert_eq!(
            events.events().first().unwrap(),
            &SimpleEnforced {
                smart_account: smart_account.clone(),
                context_rule_id: context_rule.id,
                context: EnforcedContext::CallContract(contract, fn_name),
                signer_ids: Vec::new(&e),
            }
            .to_xdr(&e, &address),
        );
    });
}

#[test]
fn set_threshold_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_threshold(&e, 3, &context_rule, &smart_account);
        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 3);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3201)")]
fn set_threshold_zero_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_threshold(&e, 0, &context_rule, &smart_account); // Invalid threshold
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        // Verify it's installed
        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 2);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        uninstall(&e, &context_rule, &smart_account);

        // Verify uninstall event
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3200)")]
fn uninstall_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        uninstall(&e, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3200)")]
fn enforce_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (addr1, addr2, _) = create_test_signers(&e);
        let authenticated_signers =
            Vec::from_array(&e, [Signer::Delegated(addr1), Signer::Delegated(addr2)]);
        let context_rule = create_test_context_rule(&e);

        let context = Context::Contract(ContractContext {
            contract: Address::generate(&e),
            fn_name: symbol_short!("test"),
            args: ().into_val(&e),
        });

        // Try to enforce without installing the policy first
        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3202)")]
fn enforce_threshold_not_met_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let (addr1, _, _) = create_test_signers(&e);
        // Only 1 signer authenticated, but threshold is 2
        let authenticated_signers = Vec::from_array(&e, [Signer::Delegated(addr1)]);
        let context_rule = create_test_context_rule(&e);

        let context = Context::Contract(ContractContext {
            contract: Address::generate(&e),
            fn_name: symbol_short!("test"),
            args: ().into_val(&e),
        });

        // Should fail because only 1 signer but threshold is 2
        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);
    });
}

#[test]
fn enforce_with_large_call_arguments_succeeds() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_test_context_rule(&e);
    let authenticated_signers = Vec::from_array(
        &e,
        [context_rule.signers.get_unchecked(0), context_rule.signers.get_unchecked(2)],
    );

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 2 };
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let mut args = Vec::new(&e);
        args.push_back(Bytes::from_slice(&e, &std::vec![0; 20_000]).into_val(&e));
        let contract = Address::generate(&e);
        let fn_name = symbol_short!("test");
        let context = Context::Contract(ContractContext {
            contract: contract.clone(),
            fn_name: fn_name.clone(),
            args,
        });

        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        let events = e.events().all();
        assert_eq!(events.events().len(), 1);
        let event = events.events().first().unwrap();
        assert_eq!(
            event,
            &SimpleEnforced {
                smart_account: smart_account.clone(),
                context_rule_id: context_rule.id,
                context: EnforcedContext::CallContract(contract, fn_name),
                signer_ids: Vec::from_array(&e, [1, 3]),
            }
            .to_xdr(&e, &address),
        );
        let event_size = WriteXdr::to_xdr(event, Limits::none()).unwrap().len();
        assert!(
            event_size < ENFORCED_EVENT_SIZE_CEILING_BYTES,
            "enforcement event is {event_size} bytes"
        );
    });
}

#[test]
fn enforce_with_maximum_external_signers_emits_compact_ids() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_max_external_signer_context_rule(&e);
    let authenticated_signers = context_rule.signers.clone();

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: MAX_SIGNERS };
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let contract = Address::generate(&e);
        let fn_name = symbol_short!("test");
        let context = Context::Contract(ContractContext {
            contract: contract.clone(),
            fn_name: fn_name.clone(),
            args: ().into_val(&e),
        });

        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        let events = e.events().all();
        assert_eq!(events.events().len(), 1);
        let event = events.events().first().unwrap();
        assert_eq!(
            event,
            &SimpleEnforced {
                smart_account: smart_account.clone(),
                context_rule_id: context_rule.id,
                context: EnforcedContext::CallContract(contract, fn_name),
                signer_ids: context_rule.signer_ids.clone(),
            }
            .to_xdr(&e, &address),
        );
        let event_size = WriteXdr::to_xdr(event, Limits::none()).unwrap().len();
        assert!(
            event_size < ENFORCED_EVENT_SIZE_CEILING_BYTES,
            "enforcement event is {event_size} bytes"
        );
    });
}
