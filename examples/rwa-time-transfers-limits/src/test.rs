extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, String, Val,
    Vec,
};
use stellar_tokens::rwa::{
    compliance::{
        modules::time_transfers_limits::{Limit, TransferCounter},
        Compliance, ComplianceHook,
    },
    identity_registry_storage::{CountryDataManager, IdentityRegistryStorage},
    utils::token_binder::TokenBinder,
};

use crate::contract::{TimeTransfersLimitsContract, TimeTransfersLimitsContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> (Address, TimeTransfersLimitsContractClient<'a>) {
    let address = e.register(TimeTransfersLimitsContract, (admin,));
    (address.clone(), TimeTransfersLimitsContractClient::new(e, &address))
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

    fn get_country_data_entries(e: &Env, _account: Address) -> Vec<Val> {
        Vec::new(e)
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
fn set_and_manage_time_transfer_limits_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let limit_a = Limit { limit_time: 60, limit_value: 100 };
    let limit_b = Limit { limit_time: 120, limit_value: 200 };
    let (_address, client) = create_client(&e, &admin);

    client.set_time_transfer_limit(&token, &limit_a);
    client.batch_set_time_transfer_limit(&token, &vec![&e, limit_b.clone()]);

    assert_eq!(client.get_time_transfer_limits(&token), vec![&e, limit_a.clone(), limit_b.clone()]);

    client.batch_remove_time_transfer_limit(&token, &vec![&e, 120_u64]);
    assert_eq!(client.get_time_transfer_limits(&token), vec![&e, limit_a.clone()]);

    client.remove_time_transfer_limit(&token, &60);
    assert_eq!(client.get_time_transfer_limits(&token).len(), 0);
}

#[test]
fn name_compliance_address_and_required_hooks_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "TimeTransfersLimitsModule"));
    assert_eq!(
        client.required_hooks(),
        vec![&e, ComplianceHook::CanTransfer, ComplianceHook::Transferred]
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
fn verify_hook_wiring_and_counters_affect_public_transfer_checks() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let sender_identity = Address::generate(&e);
    let limit = Limit { limit_time: 60, limit_value: 100 };
    let (module_address, client) = create_client(&e, &admin);
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    irs.set_identity(&sender, &sender_identity);

    client.set_compliance_address(&compliance_id);
    for hook in [ComplianceHook::CanTransfer, ComplianceHook::Transferred] {
        compliance.register_hook(&hook, &module_address);
    }

    client.verify_hook_wiring();
    client.set_identity_registry_storage(&token, &irs_id);
    client.set_time_transfer_limit(&token, &limit);
    client.pre_set_transfer_counter(
        &token,
        &sender_identity,
        &60,
        &TransferCounter { value: 90, timer: e.ledger().timestamp().saturating_add(60) },
    );

    assert!(!client.can_transfer(&sender, &recipient, &11, &token));
    assert!(client.can_transfer(&sender, &recipient, &10, &token));

    client.on_transfer(&sender, &recipient, &10, &token);
    assert!(!client.can_transfer(&sender, &recipient, &1, &token));
}
