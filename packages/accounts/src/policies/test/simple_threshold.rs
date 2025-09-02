#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::Context,
    contract,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Vec,
};

use crate::policies::simple_threshold::*;

#[contract]
struct MockContract;

fn create_test_signers(e: &Env) -> (Vec<Signer>, Address, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);
    let addr3 = Address::generate(e);

    let signers = Vec::from_array(
        e,
        [
            Signer::Native(addr1.clone()),
            Signer::Native(addr2.clone()),
            Signer::Native(addr3.clone()),
        ],
    );

    (signers, addr1, addr2, addr3)
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        assert_eq!(get_threshold(&e, &smart_account), 2);
        let stored_signers = get_signers(&e, &smart_account);
        assert_eq!(stored_signers.len(), 3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn install_zero_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 0 }; // Invalid

        install(&e, &params, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn install_empty_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let signers = Vec::new(&e); // Empty signers
        let params = SimpleThresholdInstallParams { signers, threshold: 1 };

        install(&e, &params, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn install_threshold_exceeds_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 5 }; // Exceeds 3 signers

        install(&e, &params, &smart_account);
    });
}

#[test]
fn count_authenticated_signers_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, addr1, addr2, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        let authenticated = Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
        let count = count_authenticated_signers(&e, &authenticated, &smart_account);

        assert_eq!(count, 2);
    });
}

#[test]
fn count_authenticated_signers_partial() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, addr1, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        let authenticated = Vec::from_array(&e, [Signer::Native(addr1)]);
        let count = count_authenticated_signers(&e, &authenticated, &smart_account);

        assert_eq!(count, 1);
    });
}

#[test]
fn count_authenticated_signers_with_unknown() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, addr1, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        let unknown_addr = Address::generate(&e);
        let authenticated = Vec::from_array(
            &e,
            [
                Signer::Native(addr1),
                Signer::Native(unknown_addr), // Not in policy signers
            ],
        );
        let count = count_authenticated_signers(&e, &authenticated, &smart_account);

        assert_eq!(count, 1); // Only addr1 counts
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn count_authenticated_signers_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.as_contract(&address, || {
        let authenticated = Vec::from_array(&e, [Signer::Native(Address::generate(&e))]);
        count_authenticated_signers(&e, &authenticated, &smart_account);
    });
}

#[test]
fn can_enforce_sufficient_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, addr1, addr2, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        let authenticated_signers =
            Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result = can_enforce(
            &e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        );

        assert!(result);
    });
}

#[test]
fn can_enforce_insufficient_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, addr1, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr1)]);
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result = can_enforce(
            &e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        );

        assert!(!result);
    });
}

#[test]
fn can_enforce_not_installed() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.as_contract(&address, || {
        let authenticated_signers = Vec::from_array(&e, [Signer::Native(Address::generate(&e))]);
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result = can_enforce(
            &e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        );

        assert!(!result);
    });
}

#[test]
fn enforce_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let authenticated_signers = e.as_contract(&address, || {
        let (signers, addr1, addr2, _) = create_test_signers(&e);
        let authenticated_signers =
            Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        authenticated_signers
    });

    e.as_contract(&address, || {
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        enforce(&e, &context, &context_rule_signers, &authenticated_signers, &smart_account);

        assert_eq!(e.events().all().len(), 1);
    });
}

#[test]
fn set_threshold_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        set_threshold(&e, 3, &smart_account);
        assert_eq!(get_threshold(&e, &smart_account), 3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn set_threshold_zero_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        set_threshold(&e, 0, &smart_account); // Invalid threshold
    });
}

#[test]
fn add_signer_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        let new_signer = Signer::Native(Address::generate(&e));
        add_signer(&e, &new_signer, &smart_account);

        let updated_signers = get_signers(&e, &smart_account);
        assert_eq!(updated_signers.len(), 4);
        assert!(updated_signers.contains(&new_signer));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn add_signer_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let existing_signer = e.as_contract(&address, || {
        let (signers, addr1, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);
        addr1
    });

    e.as_contract(&address, || {
        let existing_signer = Signer::Native(existing_signer);
        add_signer(&e, &existing_signer, &smart_account);

        let updated_signers = get_signers(&e, &smart_account);
        assert_eq!(updated_signers.len(), 4); // Should still be 4, not 5
    });
}

#[test]
fn remove_signer_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let signer_to_remove = e.as_contract(&address, || {
        let (signers, addr1, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);
        addr1
    });

    e.as_contract(&address, || {
        let signer_to_remove = Signer::Native(signer_to_remove);
        remove_signer(&e, &signer_to_remove, &smart_account);

        let updated_signers = get_signers(&e, &smart_account);
        assert_eq!(updated_signers.len(), 2);
        assert!(!updated_signers.contains(&signer_to_remove));
    });
}

#[test]
fn remove_signer_not_found_ignored() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        let non_existent_signer = Signer::Native(Address::generate(&e));
        remove_signer(&e, &non_existent_signer, &smart_account);

        let updated_signers = get_signers(&e, &smart_account);
        assert_eq!(updated_signers.len(), 3); // Should remain unchanged
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (signers, _, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { signers, threshold: 2 };

        install(&e, &params, &smart_account);

        // Verify it's installed
        assert_eq!(get_threshold(&e, &smart_account), 2);
    });

    e.as_contract(&address, || {
        uninstall(&e, &smart_account);
    });
}
