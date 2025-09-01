extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, vec, Address, Env};

use crate::rwa::compliance::{
    storage::{
        add_module_to, can_transfer, created, destroyed, get_modules_for_hook,
        is_module_registered, remove_module_from, transferred,
    },
    ComplianceHook, MAX_MODULES,
};

#[contract]
struct MockContract;

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
        add_module_to(&e, ComplianceHook::CanTransfer, module.clone());

        // Verify module is registered for all hooks
        assert!(is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        assert!(is_module_registered(&e, ComplianceHook::Created, module.clone()));
        assert!(is_module_registered(&e, ComplianceHook::CanTransfer, module.clone()));
        assert!(!is_module_registered(&e, ComplianceHook::Destroyed, module.clone()));

        // Verify each hook has the module
        let transfer_modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        let created_modules = get_modules_for_hook(&e, ComplianceHook::Created);
        let can_transfer_modules = get_modules_for_hook(&e, ComplianceHook::CanTransfer);
        let destroyed_modules = get_modules_for_hook(&e, ComplianceHook::Destroyed);

        assert_eq!(transfer_modules.len(), 1);
        assert_eq!(created_modules.len(), 1);
        assert_eq!(can_transfer_modules.len(), 1);
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
    });

    e.as_contract(&address, || {
        // Remove module
        remove_module_from(&e, ComplianceHook::Transferred, module.clone());

        // Verify module is no longer registered
        assert!(!is_module_registered(&e, ComplianceHook::Transferred, module.clone()));
        let modules = get_modules_for_hook(&e, ComplianceHook::Transferred);
        assert!(modules.is_empty());
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
        assert!(!is_module_registered(&e, ComplianceHook::CanTransfer, module.clone()));
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
        assert!(!is_module_registered(&e, ComplianceHook::CanTransfer, module.clone()));

        // Verify only Transfer hook has modules
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::Transferred).len(), 1);
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::Created).len(), 0);
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::Destroyed).len(), 0);
        assert_eq!(get_modules_for_hook(&e, ComplianceHook::CanTransfer).len(), 0);
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
            ComplianceHook::CanTransfer,
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
    pub fn on_transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {
        // Mock implementation - does nothing but proves it was called
    }

    pub fn on_created(_env: Env, _to: Address, _amount: i128) {
        // Mock implementation - does nothing but proves it was called
    }

    pub fn on_destroyed(_env: Env, _from: Address, _amount: i128) {
        // Mock implementation - does nothing but proves it was called
    }

    pub fn can_transfer(_env: Env, _from: Address, _to: Address, amount: i128) -> bool {
        // Mock implementation - returns true for even amounts, false for odd amounts
        amount % 2 == 0
    }
}

#[test]
fn transferred_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Add module to Transfer hook
        add_module_to(&e, ComplianceHook::Transferred, module_address.clone());

        // Execute transferred hook - should not panic
        transferred(&e, from.clone(), to.clone(), amount);

        // Test with no modules registered for other hooks
        transferred(&e, from, to, amount);
    });
}

#[test]
fn created_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let to = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Add module to Created hook
        add_module_to(&e, ComplianceHook::Created, module_address.clone());

        // Execute created hook - should not panic
        created(&e, to.clone(), amount);

        // Test with no modules registered
        let empty_to = Address::generate(&e);
        created(&e, empty_to, amount);
    });
}

#[test]
fn destroyed_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Add module to Destroyed hook
        add_module_to(&e, ComplianceHook::Destroyed, module_address.clone());

        // Execute destroyed hook - should not panic
        destroyed(&e, from.clone(), amount);

        // Test with no modules registered
        let empty_from = Address::generate(&e);
        destroyed(&e, empty_from, amount);
    });
}

#[test]
fn can_transfer_hook_execution_works() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    e.as_contract(&contract_address, || {
        // Test with no modules registered - should return true
        assert!(can_transfer(&e, from.clone(), to.clone(), amount));

        // Add module to CanTransfer hook
        add_module_to(&e, ComplianceHook::CanTransfer, module_address.clone());

        // Execute can_transfer hook with even amount - should return true
        let even_amount = 1000i128;
        assert!(can_transfer(&e, from.clone(), to.clone(), even_amount));

        // Execute can_transfer hook with odd amount - should return false
        let odd_amount = 1001i128;
        assert!(!can_transfer(&e, from.clone(), to.clone(), odd_amount));
    });
}

#[test]
fn can_transfer_returns_false_when_module_rejects() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    let module_address = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Add module to CanTransfer hook
        add_module_to(&e, ComplianceHook::CanTransfer, module_address);

        // Execute can_transfer hook with odd amount - should return false
        let odd_amount = 1001i128;
        assert!(!can_transfer(&e, from, to, odd_amount));
    });
}

#[test]
fn can_transfer_multiple_modules_all_must_pass() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    let module1 = e.register(MockComplianceModule, ());
    let module2 = e.register(MockComplianceModule, ());
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Add two identical modules to CanTransfer hook
        add_module_to(&e, ComplianceHook::CanTransfer, module1);
        add_module_to(&e, ComplianceHook::CanTransfer, module2);

        // Test with even amount - both modules should return true, so result is true
        let even_amount = 1000i128;
        assert!(can_transfer(&e, from.clone(), to.clone(), even_amount));

        // Test with odd amount - both modules should return false, so result is false
        let odd_amount = 1001i128;
        assert!(!can_transfer(&e, from, to, odd_amount));
    });
}
