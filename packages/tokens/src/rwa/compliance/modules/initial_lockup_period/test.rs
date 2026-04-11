extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger as _},
    vec, Address, Env,
};

use super::storage::{
    can_transfer, configure_lockup_period, get_internal_balance, get_locks, get_total_locked,
    on_created, on_destroyed, on_transfer, pre_set_lockup_state, set_internal_balance, set_locks,
    set_total_locked, verify_hook_wiring, LockedTokens,
};
use crate::rwa::{
    compliance::{
        modules::storage::{hooks_verified, set_compliance_address, ComplianceModuleStorageKey},
        Compliance, ComplianceHook,
    },
    utils::token_binder::TokenBinder,
};

#[contract]
struct TestModuleContract;

fn arm_hooks(e: &Env) {
    e.storage().instance().set(&ComplianceModuleStorageKey::HooksVerified, &true);
}

#[contract]
struct MockComplianceContract;

#[derive(Clone)]
#[contracttype]
enum MockComplianceStorageKey {
    Registered(ComplianceHook, Address),
}

#[contractimpl]
impl Compliance for MockComplianceContract {
    fn add_module_to(_e: &Env, _hook: ComplianceHook, _module: Address, _operator: Address) {
        unreachable!("add_module_to is not used in these tests");
    }

    fn remove_module_from(_e: &Env, _hook: ComplianceHook, _module: Address, _operator: Address) {
        unreachable!("remove_module_from is not used in these tests");
    }

    fn get_modules_for_hook(_e: &Env, _hook: ComplianceHook) -> soroban_sdk::Vec<Address> {
        unreachable!("get_modules_for_hook is not used in these tests");
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        e.storage().persistent().has(&MockComplianceStorageKey::Registered(hook, module))
    }

    fn transferred(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {
        unreachable!("transferred is not used in these tests");
    }

    fn created(_e: &Env, _to: Address, _amount: i128, _token: Address) {
        unreachable!("created is not used in these tests");
    }

    fn destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {
        unreachable!("destroyed is not used in these tests");
    }

    fn can_transfer(
        _e: &Env,
        _from: Address,
        _to: Address,
        _amount: i128,
        _token: Address,
    ) -> bool {
        unreachable!("can_transfer is not used in these tests");
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        unreachable!("can_create is not used in these tests");
    }
}

#[contractimpl]
impl TokenBinder for MockComplianceContract {
    fn linked_tokens(e: &Env) -> soroban_sdk::Vec<Address> {
        soroban_sdk::Vec::new(e)
    }

    fn bind_token(_e: &Env, _token: Address, _operator: Address) {
        unreachable!("bind_token is not used in these tests");
    }

    fn unbind_token(_e: &Env, _token: Address, _operator: Address) {
        unreachable!("unbind_token is not used in these tests");
    }
}

#[contractimpl]
impl MockComplianceContract {
    pub fn register_hook(e: &Env, hook: ComplianceHook, module: Address) {
        e.storage().persistent().set(&MockComplianceStorageKey::Registered(hook, module), &true);
    }
}

#[test]
fn verify_hook_wiring_sets_cache_when_registered() {
    let e = Env::default();
    let module_id = e.register(TestModuleContract, ());
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    for hook in [
        ComplianceHook::CanTransfer,
        ComplianceHook::Created,
        ComplianceHook::Transferred,
        ComplianceHook::Destroyed,
    ] {
        compliance.register_hook(&hook, &module_id);
    }

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        verify_hook_wiring(&e);

        assert!(hooks_verified(&e));
    });
}

#[test]
fn pre_set_lockup_state_seeds_existing_locked_balance() {
    let e = Env::default();

    let module_id = e.register(TestModuleContract, ());
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);
        arm_hooks(&e);

        pre_set_lockup_state(
            &e,
            &token,
            &wallet,
            100,
            &vec![
                &e,
                LockedTokens {
                    amount: 80,
                    release_timestamp: e.ledger().timestamp().saturating_add(60),
                },
            ],
        );

        assert_eq!(get_internal_balance(&e, &token, &wallet), 100);
        assert_eq!(get_total_locked(&e, &token, &wallet), 80);
        assert!(!can_transfer(&e, &wallet, 21, &token));
        assert!(can_transfer(&e, &wallet, 20, &token));
    });
}

#[test]
fn can_transfer_returns_true_without_locks_and_false_for_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestModuleContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        arm_hooks(&e);

        assert!(!can_transfer(&e, &wallet, -1, &token));
        assert!(can_transfer(&e, &wallet, 1_000, &token));
    });
}

#[test]
fn on_created_locks_minted_amount_when_period_is_configured() {
    let e = Env::default();
    e.ledger().set_timestamp(100);

    let module_id = e.register(TestModuleContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        configure_lockup_period(&e, &token, 60);

        on_created(&e, &wallet, 40, &token);

        let locks = get_locks(&e, &token, &wallet);
        assert_eq!(locks.len(), 1);
        let lock = locks.get(0).unwrap();
        assert_eq!(lock.amount, 40);
        assert_eq!(lock.release_timestamp, 160);
        assert_eq!(get_total_locked(&e, &token, &wallet), 40);
        assert_eq!(get_internal_balance(&e, &token, &wallet), 40);
    });
}

#[test]
fn on_transfer_consumes_unlocked_locks_before_updating_balances() {
    let e = Env::default();
    e.ledger().set_timestamp(100);

    let module_id = e.register(TestModuleContract, ());
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_internal_balance(&e, &token, &sender, 100);
        set_internal_balance(&e, &token, &recipient, 10);
        set_locks(
            &e,
            &token,
            &sender,
            &vec![
                &e,
                LockedTokens { amount: 30, release_timestamp: 90 },
                LockedTokens { amount: 40, release_timestamp: 200 },
            ],
        );
        set_total_locked(&e, &token, &sender, 70);

        on_transfer(&e, &sender, &recipient, 50, &token);

        let locks = get_locks(&e, &token, &sender);
        assert_eq!(locks.len(), 2);
        let first_lock = locks.get(0).unwrap();
        assert_eq!(first_lock.amount, 10);
        assert_eq!(first_lock.release_timestamp, 90);
        let second_lock = locks.get(1).unwrap();
        assert_eq!(second_lock.amount, 40);
        assert_eq!(second_lock.release_timestamp, 200);
        assert_eq!(get_total_locked(&e, &token, &sender), 50);
        assert_eq!(get_internal_balance(&e, &token, &sender), 50);
        assert_eq!(get_internal_balance(&e, &token, &recipient), 60);
    });
}

#[test]
fn on_destroyed_consumes_unlocked_locks_before_burning() {
    let e = Env::default();
    e.ledger().set_timestamp(100);

    let module_id = e.register(TestModuleContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_internal_balance(&e, &token, &wallet, 100);
        set_locks(
            &e,
            &token,
            &wallet,
            &vec![
                &e,
                LockedTokens { amount: 30, release_timestamp: 90 },
                LockedTokens { amount: 40, release_timestamp: 200 },
            ],
        );
        set_total_locked(&e, &token, &wallet, 70);

        on_destroyed(&e, &wallet, 50, &token);

        let locks = get_locks(&e, &token, &wallet);
        assert_eq!(locks.len(), 2);
        let first_lock = locks.get(0).unwrap();
        assert_eq!(first_lock.amount, 10);
        assert_eq!(first_lock.release_timestamp, 90);
        let second_lock = locks.get(1).unwrap();
        assert_eq!(second_lock.amount, 40);
        assert_eq!(second_lock.release_timestamp, 200);
        assert_eq!(get_total_locked(&e, &token, &wallet), 50);
        assert_eq!(get_internal_balance(&e, &token, &wallet), 50);
    });
}

#[test]
#[should_panic]
fn on_destroyed_panics_when_burn_exceeds_unlocked_balance() {
    let e = Env::default();
    e.ledger().set_timestamp(100);

    let module_id = e.register(TestModuleContract, ());
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_internal_balance(&e, &token, &wallet, 100);
        set_locks(
            &e,
            &token,
            &wallet,
            &vec![
                &e,
                LockedTokens { amount: 10, release_timestamp: 90 },
                LockedTokens { amount: 70, release_timestamp: 200 },
            ],
        );
        set_total_locked(&e, &token, &wallet, 80);

        on_destroyed(&e, &wallet, 40, &token);
    });
}
