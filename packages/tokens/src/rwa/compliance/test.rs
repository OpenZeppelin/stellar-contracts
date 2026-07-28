extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    vec, Address, Env,
};

use crate::rwa::{
    compliance::{
        storage::{
            add_module_to, created, destroyed, get_modules_for_hook, is_module_registered,
            remove_module_from, transferred,
        },
        AccountSnapshot, ComplianceHook, TransferKind, MAX_MODULES,
    },
    utils::token_binder::bind_token,
};

#[contract]
struct MockContract;

/// Builds a snapshot for the hook-execution tests. The mock module ignores
/// the balance and frozen fields, so they are left at zero.
fn snapshot(address: &Address) -> AccountSnapshot {
    AccountSnapshot { address: address.clone(), balance: 0, frozen: 0 }
}

#[test]
fn add_module_to_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Check initial state
        assert!(!is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert!(modules.is_empty());

        // Add module
        add_module_to(&e, ComplianceHook::Transferred, module.clone());

        // Verify module is registered
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert_eq!(modules.len(), 1);
        assert_eq!(modules.get(0).unwrap(), module);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn add_multiple_modules_to_same_hook_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module1 = Address::generate(&e);
    let module2 = Address::generate(&e);
    let module3 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add multiple modules to the same hook
        add_module_to(&e, ComplianceHook::Transferred, module1.clone());
        add_module_to(&e, ComplianceHook::Transferred, module2.clone());
        add_module_to(&e, ComplianceHook::Transferred, module3.clone());

        // Verify all modules are registered
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module1.clone()));
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module2.clone()));
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module3.clone()));

        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert_eq!(modules.len(), 3);
    });
}

#[test]
fn add_module_to_different_hooks_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Add same module to different hooks
        add_module_to(&e, ComplianceHook::Transferred, module.clone());
        add_module_to(&e, ComplianceHook::Created, module.clone());

        // Verify module is registered for both hooks
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        assert!(is_module_registered(&e, ComplianceHook::Created, module.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Destroyed, module.clone()));

        // Verify each hook has the module
        let transfer_modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        let created_modules = get_modules_for_hook(&e, ComplianceHook::Created);
        let destroyed_modules = get_modules_for_hook(&e, ComplianceHook::Destroyed);

        assert_eq!(transfer_modules.len(), 1);
        assert_eq!(created_modules.len(), 1);
        assert_eq!(destroyed_modules.len(), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #360)")]
fn add_module_already_registered_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Add module first time
        add_module_to(&e, ComplianceHook::Transferred, module.clone());

        // Try to add the same module again - should panic
        add_module_to(&e, ComplianceHook::Transferred, module.clone());
    });
}

#[test]
fn remove_module_from_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Add module first
        add_module_to(&e, ComplianceHook::Transferred, module.clone());
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module.clone()));

        // Remove module
        remove_module_from(&e, ComplianceHook::Transferred, module.clone());

        // Verify module is no longer registered
        assert!(!is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert!(modules.is_empty());
        // 1 ModuleAdded + 1 ModuleRemoved
        assert_eq!(e.events().all().events().len(), 2);
    });
}

#[test]
fn remove_module_from_multiple_modules_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module1 = Address::generate(&e);
    let module2 = Address::generate(&e);
    let module3 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add multiple modules
        add_module_to(&e, ComplianceHook::Transferred, module1.clone());
        add_module_to(&e, ComplianceHook::Transferred, module2.clone());
        add_module_to(&e, ComplianceHook::Transferred, module3.clone());

        // Remove middle module
        remove_module_from(&e, ComplianceHook::Transferred, module2.clone());

        // Verify correct modules remain
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module1.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Transferred, module2.clone()));
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module3.clone()));

        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert_eq!(modules.len(), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #361)")]
fn remove_module_not_registered_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Try to remove module that was never added - should panic
        remove_module_from(&e, ComplianceHook::Transferred, module.clone());
    });
}

#[test]
fn get_modules_for_hook_empty_returns_empty_vec() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Get modules for hook with no registered modules
        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert!(modules.is_empty());
    });
}

#[test]
fn is_module_registered_false_for_unregistered() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Check unregistered module
        assert!(!is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Created, module.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Destroyed, module.clone()));
    });
}

#[test]
fn hook_isolation_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Add module to Transfer hook only
        add_module_to(&e, ComplianceHook::Transferred, module.clone());

        // Verify module is only registered for Transfer hook
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Created, module.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Destroyed, module.clone()));

        // Verify only Transfer hook has modules
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::Transferred).len(), 1);
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::Created).len(), 0);
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::Destroyed).len(), 0);
    });
}

#[test]
fn module_order_preserved() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module1 = Address::generate(&e);
    let module2 = Address::generate(&e);
    let module3 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add modules in specific order
        add_module_to(&e, ComplianceHook::Transferred, module1.clone());
        add_module_to(&e, ComplianceHook::Transferred, module2.clone());
        add_module_to(&e, ComplianceHook::Transferred, module3.clone());

        // Verify order is preserved
        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert_eq!(modules.len(), 3);
        assert_eq!(modules.get(0).unwrap(), module1);
        assert_eq!(modules.get(1).unwrap(), module2);
        assert_eq!(modules.get(2).unwrap(), module3);
    });
}

#[test]
fn all_hook_types_work() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let module = Address::generate(&e);

    e.as_contract(&address, || {
        // Test all hook types
        let hook_types = vec![
            &e,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ];

        for hook_type in hook_types.iter() {
            // Add module to each hook type
            add_module_to(&e, hook_type.clone(), module.clone());

            // Verify registration
            assert!(is_module_registered(&e, hook_type.clone(), module.clone()));

            // Verify it appears in modules list
            let modules = get_modules_for_hook(&e, hook_type.clone());
            assert_eq!(modules.len(), 1);
            assert_eq!(modules.get(0).unwrap(), module);

            // Remove module
            remove_module_from(&e, hook_type.clone(), module.clone());

            // Verify removal
            assert!(!is_module_registered(&e, hook_type.clone(), module.clone()));
            assert!(get_modules_for_hook(&e, hook_type.clone()).is_empty());
        }
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #362)")]
fn add_module_exceeds_max_modules_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Add MAX_MODULES (20) modules
        (0..MAX_MODULES).for_each(|_| {
            let module = Address::generate(&e);
            add_module_to(&e, ComplianceHook::Transferred, module);
        });

        // Try to add one more module - should panic with ModuleBoundExceeded
        let extra_module = Address::generate(&e);
        add_module_to(&e, ComplianceHook::Transferred, extra_module);
    });
}

// Mock compliance module for testing hook execution
#[contract]
struct MockComplianceModule;

#[contractimpl]
impl MockComplianceModule {
    // Mock implementations: prove the hook was called and reject odd
    // amounts by panicking, mirroring how a real module enforces policy.
    pub fn on_transfer(
        _env: Env,
        _from: AccountSnapshot,
        _to: AccountSnapshot,
        amount: i128,
        _kind: TransferKind,
        _contract: Address,
    ) {
        assert!(amount % 2 == 0, "mock module rejects odd amounts");
    }

    pub fn on_created(_env: Env, _to: AccountSnapshot, amount: i128, _contract: Address) {
        assert!(amount % 2 == 0, "mock module rejects odd amounts");
    }

    pub fn on_destroyed(_env: Env, _from: AccountSnapshot, _amount: i128, _contract: Address) {
        // Mock implementation - does nothing but proves it was called
    }
}

#[test]
fn transferred_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let token_contract_address = Address::generate(&e);
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Bind token contract to compliance contract
        bind_token(&e, &token_contract_address);

        // Add module to Transfer hook
        add_module_to(&e, ComplianceHook::Transferred, module_address.clone());

        // Execute transferred hook
        transferred(
            &e,
            snapshot(&from),
            snapshot(&to),
            amount,
            TransferKind::Standard,
            token_contract_address.clone(),
        );
    });
}

#[test]
fn created_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let token_contract_address = Address::generate(&e);
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let to = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Bind token contract to compliance contract
        bind_token(&e, &token_contract_address);

        // Add module to Created hook
        add_module_to(&e, ComplianceHook::Created, module_address.clone());

        // Execute created hook
        created(&e, snapshot(&to), amount, token_contract_address.clone());
    });
}

#[test]
fn destroyed_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let token_contract_address = Address::generate(&e);
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Bind token contract to compliance contract
        bind_token(&e, &token_contract_address);

        // Add module to Destroyed hook
        add_module_to(&e, ComplianceHook::Destroyed, module_address.clone());

        // Execute destroyed hook
        destroyed(&e, snapshot(&from), amount, token_contract_address.clone());
    });
}

#[test]
#[should_panic(expected = "mock module rejects odd amounts")]
fn transferred_panics_when_module_rejects() {
    let e = Env::default();
    e.mock_all_auths();
    let token_contract_address = Address::generate(&e);
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&contract_address, || {
        bind_token(&e, &token_contract_address);
        add_module_to(&e, ComplianceHook::Transferred, module_address);

        // The mock module rejects odd amounts; its panic propagates and
        // reverts the whole operation.
        let odd_amount = 1001i128;
        transferred(
            &e,
            snapshot(&from),
            snapshot(&to),
            odd_amount,
            TransferKind::Standard,
            token_contract_address,
        );
    });
}

#[test]
#[should_panic(expected = "mock module rejects odd amounts")]
fn created_panics_when_any_module_rejects() {
    let e = Env::default();
    e.mock_all_auths();
    let token_contract_address = Address::generate(&e);
    let contract_address = e.register(MockContract, ());
    let module1 = e.register(MockComplianceModule, ());
    let module2 = e.register(MockComplianceModule, ());
    let to = Address::generate(&e);

    e.as_contract(&contract_address, || {
        bind_token(&e, &token_contract_address);
        add_module_to(&e, ComplianceHook::Created, module1);
        add_module_to(&e, ComplianceHook::Created, module2);

        // Both modules run; one rejection is enough to revert the mint.
        let odd_amount = 1001i128;
        created(&e, snapshot(&to), odd_amount, token_contract_address);
    });
}
