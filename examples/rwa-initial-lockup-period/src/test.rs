extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, String, Vec,
};
use stellar_tokens::rwa::{
    compliance::{modules::initial_lockup_period::LockedTokens, Compliance, ComplianceHook},
    utils::token_binder::TokenBinder,
};

use crate::contract::{InitialLockupPeriodContract, InitialLockupPeriodContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> (Address, InitialLockupPeriodContractClient<'a>) {
    let address = e.register(InitialLockupPeriodContract, (admin,));
    (address.clone(), InitialLockupPeriodContractClient::new(e, &address))
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
fn set_and_get_lockup_state_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let release_timestamp = e.ledger().timestamp().saturating_add(60);
    let locks = vec![
        &e,
        LockedTokens { amount: 80, release_timestamp },
        LockedTokens { amount: 10, release_timestamp: release_timestamp.saturating_add(60) },
    ];
    let (_address, client) = create_client(&e, &admin);

    client.set_lockup_period(&token, &60);
    client.pre_set_lockup_state(&token, &wallet, &100, &locks);

    assert_eq!(client.get_lockup_period(&token), 60);
    assert_eq!(client.get_total_locked(&token, &wallet), 90);
    assert_eq!(client.get_internal_balance(&token, &wallet), 100);

    let stored_locks = client.get_locked_tokens(&token, &wallet);
    assert_eq!(stored_locks.len(), 2);

    let first_lock = stored_locks.get(0).unwrap();
    assert_eq!(first_lock.amount, 80);
    assert_eq!(first_lock.release_timestamp, release_timestamp);
}

#[test]
fn name_compliance_address_and_required_hooks_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "InitialLockupPeriodModule"));
    assert_eq!(
        client.required_hooks(),
        vec![
            &e,
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ]
    );

    client.set_compliance_address(&compliance);
    assert_eq!(client.get_compliance_address(), compliance);
}

#[test]
fn set_lockup_period_uses_admin_auth_before_compliance_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_lockup_period(&token, &60);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_lockup_period_uses_compliance_auth_after_bind() {
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

    client.set_lockup_period(&token, &60);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &compliance);
}

#[test]
fn verify_hook_wiring_and_can_transfer_use_public_contract_api() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let recipient = Address::generate(&e);
    let release_timestamp = e.ledger().timestamp().saturating_add(60);
    let locks = vec![&e, LockedTokens { amount: 80, release_timestamp }];
    let (module_address, client) = create_client(&e, &admin);
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    client.set_compliance_address(&compliance_id);
    for hook in [
        ComplianceHook::CanTransfer,
        ComplianceHook::Created,
        ComplianceHook::Transferred,
        ComplianceHook::Destroyed,
    ] {
        compliance.register_hook(&hook, &module_address);
    }

    client.verify_hook_wiring();
    client.pre_set_lockup_state(&token, &wallet, &100, &locks);

    assert!(!client.can_transfer(&wallet, &recipient, &21, &token));
    assert!(client.can_transfer(&wallet, &recipient, &20, &token));
}
