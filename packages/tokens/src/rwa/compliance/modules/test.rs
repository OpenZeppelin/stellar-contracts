extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, IntoVal, Val,
    Vec,
};

use super::storage::*;
use crate::rwa::{
    compliance::{Compliance, ComplianceHook},
    identity_registry_storage::{
        CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
        IndividualCountryRelation,
    },
    utils::token_binder::TokenBinder,
};

#[contract]
struct MockModuleContract;

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

    fn get_modules_for_hook(_e: &Env, _hook: ComplianceHook) -> Vec<Address> {
        unreachable!("get_modules_for_hook is not used in these tests");
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        let key = MockComplianceStorageKey::Registered(hook, module);
        e.storage().persistent().has(&key)
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
    fn linked_tokens(e: &Env) -> Vec<Address> {
        Vec::new(e)
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
        let key = MockComplianceStorageKey::Registered(hook, module);
        e.storage().persistent().set(&key, &true);
    }
}

#[contract]
struct MockIRSContract;

#[contracttype]
#[derive(Clone)]
enum MockIRSStorageKey {
    Identity(Address),
    CountryEntries(Address),
}

#[contractimpl]
impl TokenBinder for MockIRSContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        Vec::new(e)
    }

    fn bind_token(_e: &Env, _token: Address, _operator: Address) {
        unreachable!("bind_token is not used in these tests");
    }

    fn unbind_token(_e: &Env, _token: Address, _operator: Address) {
        unreachable!("unbind_token is not used in these tests");
    }
}

#[contractimpl]
impl IdentityRegistryStorage for MockIRSContract {
    fn add_identity(
        _e: &Env,
        _account: Address,
        _identity: Address,
        _country_data_list: Vec<Val>,
        _operator: Address,
    ) {
        unreachable!("add_identity is not used in these tests");
    }

    fn remove_identity(_e: &Env, _account: Address, _operator: Address) {
        unreachable!("remove_identity is not used in these tests");
    }

    fn modify_identity(_e: &Env, _account: Address, _identity: Address, _operator: Address) {
        unreachable!("modify_identity is not used in these tests");
    }

    fn recover_identity(
        _e: &Env,
        _old_account: Address,
        _new_account: Address,
        _operator: Address,
    ) {
        unreachable!("recover_identity is not used in these tests");
    }

    fn stored_identity(e: &Env, account: Address) -> Address {
        e.storage()
            .persistent()
            .get(&MockIRSStorageKey::Identity(account.clone()))
            .unwrap_or(account)
    }
}

#[contractimpl]
impl CountryDataManager for MockIRSContract {
    fn add_country_data_entries(
        _e: &Env,
        _account: Address,
        _country_data_list: Vec<Val>,
        _operator: Address,
    ) {
        unreachable!("add_country_data_entries is not used in these tests");
    }

    fn modify_country_data(
        _e: &Env,
        _account: Address,
        _index: u32,
        _country_data: Val,
        _operator: Address,
    ) {
        unreachable!("modify_country_data is not used in these tests");
    }

    fn delete_country_data(_e: &Env, _account: Address, _index: u32, _operator: Address) {
        unreachable!("delete_country_data is not used in these tests");
    }

    fn get_country_data_entries(e: &Env, account: Address) -> Vec<Val> {
        let entries: Vec<CountryData> = e
            .storage()
            .persistent()
            .get(&MockIRSStorageKey::CountryEntries(account))
            .unwrap_or_else(|| Vec::new(e));

        Vec::from_iter(e, entries.iter().map(|entry| entry.into_val(e)))
    }
}

fn sample_country_entry() -> CountryData {
    CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(276)),
        metadata: None,
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn verify_required_hooks_panics_when_unconfigured() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());

    e.as_contract(&module_id, || {
        verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);
    });
}

#[test]
fn verify_required_hooks_sets_cache_when_registered() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    compliance.register_hook(&ComplianceHook::CanTransfer, &module_id);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);

        assert!(hooks_verified(&e));
    });
}

#[test]
fn verify_required_hooks_returns_early_when_cached() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let compliance_id = e.register(MockComplianceContract, ());

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);
        e.storage().instance().set(&ComplianceModuleStorageKey::HooksVerified, &true);

        verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);

        assert!(hooks_verified(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #398)")]
fn verify_required_hooks_missing_required_hook_panics_with_contract_error() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let compliance_id = e.register(MockComplianceContract, ());

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);
    });
}

#[test]
fn get_irs_client_returns_working_client_for_configured_token() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let account = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);

        let client = get_irs_client(&e, &token);
        assert_eq!(client.stored_identity(&account), account);
        assert_eq!(get_irs_country_data_entries(&e, &token, &account).len(), 0);
    });
}

#[test]
fn get_irs_country_data_entries_returns_typed_entries() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let token = Address::generate(&e);
    let account = Address::generate(&e);
    let entries = vec![&e, sample_country_entry()];

    e.as_contract(&irs_id, || {
        e.storage().persistent().set(&MockIRSStorageKey::CountryEntries(account.clone()), &entries);
    });

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);

        assert_eq!(get_irs_country_data_entries(&e, &token, &account), entries);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #397)")]
fn get_irs_client_panics_when_not_configured() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        let _ = get_irs_client(&e, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn set_compliance_address_panics_with_contract_error_when_already_set() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let compliance_id = e.register(MockComplianceContract, ());

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);
        set_compliance_address(&e, &compliance_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn get_compliance_address_panics_when_not_configured() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());

    e.as_contract(&module_id, || {
        let _ = get_compliance_address(&e);
    });
}

#[test]
fn get_compliance_address_returns_configured_address() {
    let e = Env::default();
    let module_id = e.register(MockModuleContract, ());
    let compliance_id = e.register(MockComplianceContract, ());

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        assert_eq!(get_compliance_address(&e), compliance_id);
    });
}

#[test]
fn panicking_math_helpers_return_expected_values() {
    let e = Env::default();

    assert_eq!(add_i128_or_panic(&e, 2, 3), 5);
    assert_eq!(sub_i128_or_panic(&e, 7, 4), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #392)")]
fn add_i128_or_panic_panics_on_overflow() {
    let e = Env::default();

    let _ = add_i128_or_panic(&e, i128::MAX, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #393)")]
fn sub_i128_or_panic_panics_on_underflow() {
    let e = Env::default();

    let _ = sub_i128_or_panic(&e, i128::MIN, 1);
}
