extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env,
};

use super::storage::{
    can_transfer, get_internal_balance, get_total_locked, pre_set_lockup_state, verify_hook_wiring,
    LockedTokens,
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
