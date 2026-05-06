extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, String, Val,
    Vec,
};
use stellar_tokens::rwa::{
    compliance::{Compliance, ComplianceHook},
    identity_registry_storage::IdentityRegistryStorage,
    utils::token_binder::TokenBinder,
};

use crate::contract::{MaxBalanceContract, MaxBalanceContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> (Address, MaxBalanceContractClient<'a>) {
    let address = e.register(MaxBalanceContract, (admin,));
    (address.clone(), MaxBalanceContractClient::new(e, &address))
}

#[contract]
struct MockIRSContract;

#[contracttype]
#[derive(Clone)]
enum MockIRSStorageKey {
    Identity(Address),
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
impl MockIRSContract {
    pub fn set_identity(e: &Env, account: Address, identity: Address) {
        e.storage().persistent().set(&MockIRSStorageKey::Identity(account), &identity);
    }
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
fn set_and_get_max_balance_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_max_balance(&token, &100);

    assert_eq!(client.get_max_balance(&token), 100);
}

#[test]
fn pre_set_identity_balances_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let identity_a = Address::generate(&e);
    let identity_b = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.pre_set_identity_balance(&token, &identity_a, &40);
    client.batch_pre_set_identity_balances(
        &token,
        &vec![&e, identity_a.clone(), identity_b.clone()],
        &vec![&e, 50_i128, 20_i128],
    );

    assert_eq!(client.get_investor_balance(&token, &identity_a), 50);
    assert_eq!(client.get_investor_balance(&token, &identity_b), 20);
}

#[test]
fn name_compliance_address_and_required_hooks_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "MaxBalanceModule"));
    assert_eq!(
        client.required_hooks(),
        vec![
            &e,
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ]
    );

    client.set_compliance_address(&compliance);
    assert_eq!(client.get_compliance_address(), compliance);
}

#[test]
fn set_identity_registry_storage_uses_admin_auth_before_compliance_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let irs = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_identity_registry_storage(&token, &irs);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_identity_registry_storage_uses_compliance_auth_after_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let irs = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);

    client.set_identity_registry_storage(&token, &irs);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &compliance);
}

#[test]
fn can_create_and_can_transfer_use_identity_caps() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let sender_identity = Address::generate(&e);
    let recipient_identity = Address::generate(&e);
    let (module_address, client) = create_client(&e, &admin);
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    irs.set_identity(&sender, &sender_identity);
    irs.set_identity(&recipient, &recipient_identity);

    client.set_compliance_address(&compliance_id);
    for hook in [
        ComplianceHook::CanTransfer,
        ComplianceHook::CanCreate,
        ComplianceHook::Transferred,
        ComplianceHook::Created,
        ComplianceHook::Destroyed,
    ] {
        compliance.register_hook(&hook, &module_address);
    }

    client.verify_hook_wiring();
    client.set_identity_registry_storage(&token, &irs_id);
    client.set_max_balance(&token, &100);
    client.pre_set_identity_balance(&token, &recipient_identity, &60);

    assert!(!client.can_create(&recipient, &50, &token));
    assert!(client.can_create(&recipient, &40, &token));
    assert!(!client.can_transfer(&sender, &recipient, &50, &token));
    assert!(client.can_transfer(&sender, &recipient, &40, &token));
}
