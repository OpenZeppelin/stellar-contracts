extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, Val, Vec,
};

use super::{
    storage::{set_id_balance, set_max_balance},
    *,
};
use crate::rwa::{
    compliance::{
        modules::storage::{
            hooks_verified, set_compliance_address, set_irs_address, ComplianceModuleStorageKey,
        },
        Compliance, ComplianceHook,
    },
    identity_registry_storage::{CountryDataManager, IdentityRegistryStorage},
    utils::token_binder::TokenBinder,
};

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

#[contract]
struct TestMaxBalanceContract;

#[contractimpl(contracttrait)]
impl MaxBalance for TestMaxBalanceContract {
    fn set_compliance_address(_e: &Env, _compliance: Address) {
        unreachable!("set_compliance_address is not used in these tests");
    }
}

fn arm_hooks(e: &Env) {
    e.storage().instance().set(&ComplianceModuleStorageKey::HooksVerified, &true);
}

#[test]
fn verify_hook_wiring_sets_cache_when_registered() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    for hook in [
        ComplianceHook::CanTransfer,
        ComplianceHook::CanCreate,
        ComplianceHook::Transferred,
        ComplianceHook::Created,
        ComplianceHook::Destroyed,
    ] {
        compliance.register_hook(&hook, &module_id);
    }

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        <TestMaxBalanceContract as MaxBalance>::verify_hook_wiring(&e);

        assert!(hooks_verified(&e));
    });
}

#[test]
fn can_create_rejects_mint_when_cap_would_be_exceeded() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let recipient = Address::generate(&e);
    let recipient_identity = Address::generate(&e);

    irs.set_identity(&recipient, &recipient_identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        arm_hooks(&e);
        set_max_balance(&e, &token, 100);
        set_id_balance(&e, &token, &recipient_identity, 60);

        assert!(!<TestMaxBalanceContract as MaxBalance>::can_create(
            &e,
            recipient.clone(),
            50,
            token.clone(),
        ));
        assert!(<TestMaxBalanceContract as MaxBalance>::can_create(
            &e,
            recipient,
            40,
            token.clone(),
        ));
    });
}

#[test]
fn can_transfer_checks_distinct_recipient_identity_balance() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let sender_identity = Address::generate(&e);
    let recipient_identity = Address::generate(&e);

    irs.set_identity(&sender, &sender_identity);
    irs.set_identity(&recipient, &recipient_identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        arm_hooks(&e);
        set_max_balance(&e, &token, 100);
        set_id_balance(&e, &token, &recipient_identity, 60);

        assert!(!<TestMaxBalanceContract as MaxBalance>::can_transfer(
            &e,
            sender.clone(),
            recipient.clone(),
            50,
            token.clone(),
        ));
        assert!(<TestMaxBalanceContract as MaxBalance>::can_transfer(
            &e,
            sender,
            recipient,
            40,
            token.clone(),
        ));
    });
}

#[test]
fn can_create_allows_without_cap_and_rejects_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let token = Address::generate(&e);
    let recipient = Address::generate(&e);
    let recipient_identity = Address::generate(&e);

    irs.set_identity(&recipient, &recipient_identity);

    e.as_contract(&module_id, || {
        set_irs_address(&e, &token, &irs_id);
        arm_hooks(&e);
        set_id_balance(&e, &token, &recipient_identity, 500);

        assert!(<TestMaxBalanceContract as MaxBalance>::can_create(
            &e,
            recipient.clone(),
            1_000,
            token.clone(),
        ));
        assert!(!<TestMaxBalanceContract as MaxBalance>::can_create(
            &e,
            recipient,
            -1,
            token.clone(),
        ));
    });
}

#[test]
fn can_create_rejects_negative_amount_before_requiring_irs() {
    let e = Env::default();
    let module_id = e.register(TestMaxBalanceContract, ());
    let token = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&module_id, || {
        arm_hooks(&e);

        assert!(!<TestMaxBalanceContract as MaxBalance>::can_create(&e, recipient, -1, token,));
    });
}
