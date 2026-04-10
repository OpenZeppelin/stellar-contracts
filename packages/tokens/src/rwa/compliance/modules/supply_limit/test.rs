extern crate std;

use soroban_sdk::{contract, contractimpl, contracttype, testutils::Address as _, Address, Env};

use super::storage::{
    can_create, configure_supply_limit, get_internal_supply, on_created, on_destroyed,
    pre_set_supply, verify_hook_wiring,
};
use crate::rwa::{
    compliance::{
        modules::storage::{hooks_verified, set_compliance_address, ComplianceModuleStorageKey},
        Compliance, ComplianceHook,
    },
    utils::token_binder::TokenBinder,
};

#[contract]
struct MockComplianceContract;

#[contracttype]
#[derive(Clone)]
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

#[contract]
struct TestSupplyLimitContract;

fn arm_hooks(e: &Env) {
    e.storage().instance().set(&ComplianceModuleStorageKey::HooksVerified, &true);
}

#[test]
fn verify_hook_wiring_sets_cache_when_registered() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    for hook in [ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed] {
        compliance.register_hook(&hook, &module_id);
    }

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        verify_hook_wiring(&e);

        assert!(hooks_verified(&e));
    });
}

#[test]
fn get_supply_limit_returns_zero_when_unconfigured() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert_eq!(super::storage::get_supply_limit(&e, &token), 0);
    });
}

#[test]
fn can_create_allows_when_limit_is_unset_and_rejects_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        arm_hooks(&e);

        assert!(can_create(&e, 100, &token));
        assert!(!can_create(&e, -1, &token));
    });
}

#[test]
fn hooks_update_internal_supply_and_cap_future_mints() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        arm_hooks(&e);
        configure_supply_limit(&e, &token, 100);

        assert!(can_create(&e, 80, &token));
        on_created(&e, 80, &token);
        assert_eq!(get_internal_supply(&e, &token), 80);

        assert!(!can_create(&e, 30, &token));

        on_destroyed(&e, 20, &token);
        assert_eq!(get_internal_supply(&e, &token), 60);
        assert!(can_create(&e, 40, &token));
    });
}

#[test]
fn pre_set_internal_supply_seeds_existing_supply_for_cap_checks() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        arm_hooks(&e);
        configure_supply_limit(&e, &token, 100);
        pre_set_supply(&e, &token, 90);

        assert_eq!(get_internal_supply(&e, &token), 90);
        assert!(!can_create(&e, 11, &token));
        assert!(can_create(&e, 10, &token));
    });
}
