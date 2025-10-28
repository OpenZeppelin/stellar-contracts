extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext, ContractExecutable, CreateContractHostFnContext},
    contract, symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, BytesN, Env, IntoVal, Vec,
};

use crate::{
    policies::spending_limit::*,
    smart_account::{ContextRule, ContextRuleType, Signer},
};

#[contract]
struct MockContract;

fn create_signers(e: &Env) -> (Address, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);
    let addr3 = Address::generate(e);

    (addr1, addr2, addr3)
}

fn create_context_rule(e: &Env) -> ContextRule {
    let (addr1, addr2, addr3) = create_signers(e);
    let mut signers = Vec::new(e);
    signers.push_back(Signer::Delegated(addr1));
    signers.push_back(Signer::Delegated(addr2));
    signers.push_back(Signer::Delegated(addr3));
    let policies = Vec::new(e);
    ContextRule {
        id: 1,
        context_type: ContextRuleType::Default,
        name: soroban_sdk::String::from_str(e, "rule"),
        signers,
        policies,
        valid_until: None,
    }
}

fn create_transfer_context(e: &Env, amount: i128) -> Context {
    let contract_address = Address::generate(e);
    let from = Address::generate(e);
    let to = Address::generate(e);

    let mut args = Vec::new(e);
    args.push_back(from.into_val(e));
    args.push_back(to.into_val(e));
    args.push_back(amount.into_val(e));

    Context::Contract(ContractContext {
        contract: contract_address,
        fn_name: symbol_short!("transfer"),
        args,
    })
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_context_rule(&e);
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);

        let data = get_spending_limit_data(&e, context_rule.id, &smart_account);
        assert_eq!(data.spending_limit, 1_000_000);
        assert_eq!(data.period_ledgers, 100);
        assert_eq!(data.spending_history.len(), 0);
        assert_eq!(data.cached_total_spent, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3222)")]
fn install_invalid_spending_limit() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_context_rule(&e);
        let params = SpendingLimitAccountParams {
            spending_limit: 0, // Invalid: must be positive
            period_ledgers: 100,
        };

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3222)")]
fn install_invalid_period() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_context_rule(&e);
        let params = SpendingLimitAccountParams {
            spending_limit: 1_000_000,
            period_ledgers: 0, // Invalid: must be positive
        };

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
fn can_enforce_within_limit() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
    let context = create_transfer_context(&e, 500_000);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);

        assert!(can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));
    });
}

#[test]
fn can_enforce_exceeds_limit() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
    let context = create_transfer_context(&e, 1_500_000);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);

        assert!(!can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));
    });
}

#[test]
fn can_enforce_no_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
    let context = create_transfer_context(&e, 1_500_000);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);

        assert!(!can_enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account));
    });
}

#[test]
fn can_enforce_non_transfer_context() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);

        let contract_address = Address::generate(&e);
        let args = Vec::new(&e);
        let context = Context::Contract(ContractContext {
            contract: contract_address,
            fn_name: symbol_short!("deploy"),
            args,
        });

        assert!(!can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));
    });
}

#[test]
fn enforce_within_limit() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 500_000);

        // First verify that can_enforce returns true
        assert!(can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));

        // Check initial state - should be empty
        let initial_data = get_spending_limit_data(&e, context_rule.id, &smart_account);
        assert!(initial_data.spending_history.is_empty());

        enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);

        // Check that spending was recorded
        let data = get_spending_limit_data(&e, context_rule.id, &smart_account);

        // If this fails, the enforce function didn't save the spending entry
        assert!(!data.spending_history.is_empty());
        assert_eq!(data.spending_history.get(0).unwrap().amount, 500_000);
        assert_eq!(data.cached_total_spent, 500_000);

        // Check event was emitted
        assert!(!e.events().all().is_empty());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3221)")]
fn enforce_exceeds_limit() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let context = create_transfer_context(&e, 1_500_000);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_no_singers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let context = create_transfer_context(&e, 1_500_000);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account);
    });
}

#[test]
fn rolling_window_functionality() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    // Install policy
    e.mock_all_auths();
    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
        install(&e, &params, &context_rule, &smart_account);
    });
    e.ledger().with_mut(|li| {
        li.sequence_number = 1000;
    });

    // First transaction: 600,000
    e.as_contract(&address, || {
        let context1 = create_transfer_context(&e, 600_000);
        enforce(&e, &context1, &context_rule.signers, &context_rule, &smart_account);
    });

    // Second transaction: 300,000 (should succeed, total = 900,000)
    e.as_contract(&address, || {
        let context2 = create_transfer_context(&e, 300_000);
        enforce(&e, &context2, &context_rule.signers, &context_rule, &smart_account);
    });

    // Move forward in time but within the rolling window
    e.ledger().with_mut(|li| {
        li.sequence_number = 1050; // 50 ledgers later, still within 100 ledger
                                   // window
    });

    e.as_contract(&address, || {
        // 200,000 (should fail, total would be 1,100,000)
        let context3 = create_transfer_context(&e, 200_000);
        // Verify that can_enforce returns false for this transaction
        assert!(!can_enforce(&e, &context3, &context_rule.signers, &context_rule, &smart_account));

        // But 100,000 is fine
        let context4 = create_transfer_context(&e, 100_000);
        enforce(&e, &context4, &context_rule.signers, &context_rule, &smart_account);
    });

    // Move forward beyond the rolling window
    e.ledger().with_mut(|li| {
        li.sequence_number = 1150; // 150 ledgers later, first 2 transactions
                                   // should be outside window
    });

    // Now the 900,000 transaction should succeed
    e.as_contract(&address, || {
        let context5 = create_transfer_context(&e, 900_000);
        enforce(&e, &context5, &context_rule.signers, &context_rule, &smart_account);

        // Check that old entries were cleaned up
        let data = get_spending_limit_data(&e, context_rule.id, &smart_account);
        // After cleanup, should have second and third transactions
        assert_eq!(data.spending_history.len(), 2);

        // Verify the most recent transaction is the 900,000 one
        let last_entry = data.spending_history.get(data.spending_history.len() - 1).unwrap();
        assert_eq!(last_entry.amount, 900_000);
    });
}

#[test]
fn multiple_transactions_within_period() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    // Install policy
    e.mock_all_auths();
    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
        install(&e, &params, &context_rule, &smart_account);
    });

    // Make several small transactions
    for i in 1..=5 {
        e.mock_all_auths();
        e.as_contract(&address, || {
            e.ledger().with_mut(|li| {
                li.sequence_number = 1000 + i;
            });
            let context = create_transfer_context(&e, 150_000);
            enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);
        });
    }

    // Check total spent: 750,000, should be within limit
    e.as_contract(&address, || {
        let data = get_spending_limit_data(&e, context_rule.id, &smart_account);
        assert_eq!(data.spending_history.len(), 5);
    });

    // Try one more transaction that would exceed the limit
    e.ledger().with_mut(|li| {
        li.sequence_number = 1006;
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 300_000);
        // Verify that can_enforce returns false for this transaction
        assert!(!can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));
    });
}

#[test]
fn set_spending_limit_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        // Update the spending limit
        set_spending_limit(&e, 2_000_000, &context_rule, &smart_account);

        let data = get_spending_limit_data(&e, context_rule.id, &smart_account);
        assert_eq!(data.spending_limit, 2_000_000);
        assert_eq!(data.period_ledgers, 100); // Should remain unchanged
        assert_eq!(data.cached_total_spent, 0); // Should remain unchanged
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3222)")]
fn set_invalid_spending_limit() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        // Try to set invalid spending limit
        set_spending_limit(&e, 0, &context_rule, &smart_account);
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);

        // Verify installation
        let data = get_spending_limit_data(&e, context_rule.id, &smart_account);
        assert_eq!(data.spending_limit, 1_000_000);
    });

    e.as_contract(&address, || {
        // Uninstall
        uninstall(&e, &context_rule, &smart_account);

        // Verify uninstallation - can_enforce should return false
        let context = create_transfer_context(&e, 100_000);
        assert!(!can_enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3220)")]
fn get_spending_limit_data_not_installed() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.as_contract(&address, || {
        // Try to get data without installing first
        get_spending_limit_data(&e, context_rule.id, &smart_account);
    });
}

#[test]
fn can_enforce_not_installed() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 500_000);

        // Should return false when policy is not installed
        assert!(!can_enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account));
    });
}

#[test]
fn can_enforce_invalid_amount_arg() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);

        let contract_address = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let mut args = Vec::new(&e);
        args.push_back(from.into_val(&e));
        args.push_back(to.into_val(&e));
        // Push an invalid type for the amount
        args.push_back(symbol_short!("invalid").into_val(&e));

        let context = Context::Contract(ContractContext {
            contract: contract_address,
            fn_name: symbol_short!("transfer"),
            args,
        });

        assert!(!can_enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account));
    });
}

#[test]
fn can_enforce_missing_amount_arg() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);

        let contract_address = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let mut args = Vec::new(&e);
        args.push_back(from.into_val(&e));
        args.push_back(to.into_val(&e));
        // Do not push the amount argument

        let context = Context::Contract(ContractContext {
            contract: contract_address,
            fn_name: symbol_short!("transfer"),
            args,
        });

        assert!(!can_enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account));
    });
}

#[test]
fn can_enforce_on_non_contract_context() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = Context::CreateContractHostFn(CreateContractHostFnContext {
            salt: BytesN::from_array(&e, &[1u8; 32]),
            executable: ContractExecutable::Wasm(BytesN::from_array(&e, &[1u8; 32])),
        });

        assert!(!can_enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_non_transfer_context_errors() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        // Try to enforce with a non-transfer context (using a different function name)
        let contract_address = Address::generate(&e);
        let args = Vec::new(&e);
        let context = Context::Contract(ContractContext {
            contract: contract_address,
            fn_name: symbol_short!("deploy"),
            args,
        });

        enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_on_non_contract_context_errors() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = Context::CreateContractHostFn(CreateContractHostFnContext {
            salt: BytesN::from_array(&e, &[1u8; 32]),
            executable: ContractExecutable::Wasm(BytesN::from_array(&e, &[1u8; 32])),
        });

        enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_invalid_amount_arg_errors() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let contract_address = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let mut args = Vec::new(&e);
        args.push_back(from.into_val(&e));
        args.push_back(to.into_val(&e));
        // Push an invalid type for the amount
        args.push_back(symbol_short!("invalid").into_val(&e));

        let context = Context::Contract(ContractContext {
            contract: contract_address,
            fn_name: symbol_short!("transfer"),
            args,
        });

        enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_missing_amount_arg_errors() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let params = SpendingLimitAccountParams { spending_limit: 1_000_000, period_ledgers: 100 };

    e.mock_all_auths();

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let contract_address = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let mut args = Vec::new(&e);
        args.push_back(from.into_val(&e));
        args.push_back(to.into_val(&e));
        // Do not push the amount argument

        let context = Context::Contract(ContractContext {
            contract: contract_address,
            fn_name: symbol_short!("transfer"),
            args,
        });

        enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3224)")]
fn enforce_history_capacity_exceeded() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let context = create_transfer_context(&e, 1);

    e.mock_all_auths();

    e.as_contract(&address, || {
        // Install with a very long period so entries don't expire
        let params =
            SpendingLimitAccountParams { spending_limit: i128::MAX, period_ledgers: 1_000_000 };
        install(&e, &params, &context_rule, &smart_account);
    });

    // Fill up the history to MAX_HISTORY_ENTRIES
    for i in 0..MAX_HISTORY_ENTRIES {
        e.ledger().with_mut(|li| {
            li.sequence_number = 1000 + i;
        });
        e.as_contract(&address, || {
            enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);
        });
    }

    // This should panic with HistoryCapacityExceeded
    e.ledger().with_mut(|li| {
        li.sequence_number = MAX_HISTORY_ENTRIES + 1000;
    });
    e.as_contract(&address, || {
        assert!(!can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));
        enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);
    });
}

#[test]
fn history_capacity_allows_new_transaction_after_cleanup() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);
    let context = create_transfer_context(&e, 1);

    e.mock_all_auths();

    e.as_contract(&address, || {
        // Install with a short period so entries expire
        let params = SpendingLimitAccountParams { spending_limit: i128::MAX, period_ledgers: 100 };
        install(&e, &params, &context_rule, &smart_account);
    });

    // Fill up the history to MAX_HISTORY_ENTRIES
    for i in 0..MAX_HISTORY_ENTRIES {
        e.ledger().with_mut(|li| {
            li.sequence_number = 1000 + i;
        });
        e.as_contract(&address, || {
            enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);
        });
    }

    // Move forward beyond the period so old entries expire
    e.ledger().with_mut(|li| {
        li.sequence_number = MAX_HISTORY_ENTRIES + 1001;
    });

    // This should succeed because old entries will be cleaned up
    e.as_contract(&address, || {
        assert!(can_enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account));
        enforce(&e, &context, &context_rule.signers, &context_rule, &smart_account);
    });
}
