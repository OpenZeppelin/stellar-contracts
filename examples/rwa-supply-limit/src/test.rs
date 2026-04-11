extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, String, Vec,
};
use stellar_tokens::rwa::{
    compliance::{Compliance, ComplianceHook},
    utils::token_binder::TokenBinder,
};

use crate::contract::{SupplyLimitContract, SupplyLimitContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> (Address, SupplyLimitContractClient<'a>) {
    let address = e.register(SupplyLimitContract, (admin,));
    (address.clone(), SupplyLimitContractClient::new(e, &address))
}

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
        e.storage().persistent().set(&MockComplianceStorageKey::Registered(hook, module), &true);
    }
}

#[test]
fn set_supply_limit_and_pre_set_supply_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_supply_limit(&token, &100);
    client.pre_set_supply(&token, &60);

    assert_eq!(client.get_supply_limit(&token), 100);
    assert_eq!(client.get_internal_supply(&token), 60);
}

#[test]
fn name_compliance_address_and_required_hooks_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "SupplyLimitModule"));
    assert_eq!(
        client.required_hooks(),
        vec![&e, ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed,]
    );

    client.set_compliance_address(&compliance);
    assert_eq!(client.get_compliance_address(), compliance);
}

#[test]
fn set_supply_limit_uses_admin_auth_before_compliance_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_supply_limit(&token, &100);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_supply_limit_uses_compliance_auth_after_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);

    client.set_supply_limit(&token, &100);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &compliance);
}

#[test]
fn can_create_and_hooks_update_internal_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let account = Address::generate(&e);
    let (module_address, client) = create_client(&e, &admin);
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    client.set_compliance_address(&compliance_id);
    for hook in [ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed] {
        compliance.register_hook(&hook, &module_address);
    }

    client.verify_hook_wiring();
    client.set_supply_limit(&token, &100);

    assert!(client.can_create(&account, &80, &token));

    client.on_created(&account, &80, &token);
    assert_eq!(client.get_internal_supply(&token), 80);
    assert!(!client.can_create(&account, &30, &token));

    client.on_destroyed(&account, &20, &token);
    assert_eq!(client.get_internal_supply(&token), 60);
    assert!(client.can_create(&account, &40, &token));
}
