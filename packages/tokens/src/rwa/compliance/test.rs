extern crate std;

use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env};

use crate::rwa::compliance::{
    storage::{add_module_to, get_modules_for_hook, is_module_registered, remove_module_from},
    HookType,
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
        assert!(!is_module_registered(&e, HookType::Transfer, module.clone()));
        let modules = get_modules_for_hook(&e, HookType::Transfer);
        assert!(modules.is_empty());

        // Add module
        add_module_to(&e, HookType::Transfer, module.clone());

        // Verify module is registered
        assert!(is_module_registered(&e, HookType::Transfer, module.clone()));
        let modules = get_modules_for_hook(&e, HookType::Transfer);
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
        add_module_to(&e, HookType::Transfer, module1.clone());
        add_module_to(&e, HookType::Transfer, module2.clone());
        add_module_to(&e, HookType::Transfer, module3.clone());

        // Verify all modules are registered
        assert!(is_module_registered(&e, HookType::Transfer, module1.clone()));
        assert!(is_module_registered(&e, HookType::Transfer, module2.clone()));
        assert!(is_module_registered(&e, HookType::Transfer, module3.clone()));

        let modules = get_modules_for_hook(&e, HookType::Transfer);
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
        add_module_to(&e, HookType::Transfer, module.clone());
        add_module_to(&e, HookType::Created, module.clone());
        add_module_to(&e, HookType::CanTransfer, module.clone());

        // Verify module is registered for all hooks
        assert!(is_module_registered(&e, HookType::Transfer, module.clone()));
        assert!(is_module_registered(&e, HookType::Created, module.clone()));
        assert!(is_module_registered(&e, HookType::CanTransfer, module.clone()));
        assert!(!is_module_registered(&e, HookType::Destroyed, module.clone()));

        // Verify each hook has the module
        let transfer_modules = get_modules_for_hook(&e, HookType::Transfer);
        let created_modules = get_modules_for_hook(&e, HookType::Created);
        let can_transfer_modules = get_modules_for_hook(&e, HookType::CanTransfer);
        let destroyed_modules = get_modules_for_hook(&e, HookType::Destroyed);

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
        add_module_to(&e, HookType::Transfer, module.clone());

        // Try to add the same module again - should panic
        add_module_to(&e, HookType::Transfer, module.clone());
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
        add_module_to(&e, HookType::Transfer, module.clone());
        assert!(is_module_registered(&e, HookType::Transfer, module.clone()));
    });

    e.as_contract(&address, || {
        // Remove module
        remove_module_from(&e, HookType::Transfer, module.clone());

        // Verify module is no longer registered
        assert!(!is_module_registered(&e, HookType::Transfer, module.clone()));
        let modules = get_modules_for_hook(&e, HookType::Transfer);
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
        add_module_to(&e, HookType::Transfer, module1.clone());
        add_module_to(&e, HookType::Transfer, module2.clone());
        add_module_to(&e, HookType::Transfer, module3.clone());

        // Remove middle module
        remove_module_from(&e, HookType::Transfer, module2.clone());

        // Verify correct modules remain
        assert!(is_module_registered(&e, HookType::Transfer, module1.clone()));
        assert!(!is_module_registered(&e, HookType::Transfer, module2.clone()));
        assert!(is_module_registered(&e, HookType::Transfer, module3.clone()));

        let modules = get_modules_for_hook(&e, HookType::Transfer);
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
        remove_module_from(&e, HookType::Transfer, module.clone());
    });
}

#[test]
fn get_modules_for_hook_empty_returns_empty_vec() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Get modules for hook with no registered modules
        let modules = get_modules_for_hook(&e, HookType::Transfer);
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
        assert!(!is_module_registered(&e, HookType::Transfer, module.clone()));
        assert!(!is_module_registered(&e, HookType::Created, module.clone()));
        assert!(!is_module_registered(&e, HookType::Destroyed, module.clone()));
        assert!(!is_module_registered(&e, HookType::CanTransfer, module.clone()));
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
        add_module_to(&e, HookType::Transfer, module.clone());

        // Verify module is only registered for Transfer hook
        assert!(is_module_registered(&e, HookType::Transfer, module.clone()));
        assert!(!is_module_registered(&e, HookType::Created, module.clone()));
        assert!(!is_module_registered(&e, HookType::Destroyed, module.clone()));
        assert!(!is_module_registered(&e, HookType::CanTransfer, module.clone()));

        // Verify only Transfer hook has modules
        assert_eq!(get_modules_for_hook(&e, HookType::Transfer).len(), 1);
        assert_eq!(get_modules_for_hook(&e, HookType::Created).len(), 0);
        assert_eq!(get_modules_for_hook(&e, HookType::Destroyed).len(), 0);
        assert_eq!(get_modules_for_hook(&e, HookType::CanTransfer).len(), 0);
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
        add_module_to(&e, HookType::Transfer, module1.clone());
        add_module_to(&e, HookType::Transfer, module2.clone());
        add_module_to(&e, HookType::Transfer, module3.clone());

        // Verify order is preserved
        let modules = get_modules_for_hook(&e, HookType::Transfer);
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
            HookType::Transfer,
            HookType::Created,
            HookType::Destroyed,
            HookType::CanTransfer,
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
