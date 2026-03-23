extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, Val, Vec,
};

use super::*;
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
struct TestTimeTransfersLimitsContract;

#[contractimpl(contracttrait)]
impl TimeTransfersLimits for TestTimeTransfersLimitsContract {
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
    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let compliance_id = e.register(MockComplianceContract, ());
    let compliance = MockComplianceContractClient::new(&e, &compliance_id);

    for hook in [ComplianceHook::CanTransfer, ComplianceHook::Transferred] {
        compliance.register_hook(&hook, &module_id);
    }

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance_id);

        <TestTimeTransfersLimitsContract as TimeTransfersLimits>::verify_hook_wiring(&e);

        assert!(hooks_verified(&e));
    });
}

#[test]
fn pre_set_transfer_counter_blocks_transfers_within_active_window() {
    let e = Env::default();
    e.mock_all_auths();

    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let irs_id = e.register(MockIRSContract, ());
    let irs = MockIRSContractClient::new(&e, &irs_id);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let sender_identity = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = TestTimeTransfersLimitsContractClient::new(&e, &module_id);

    irs.set_identity(&sender, &sender_identity);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);
        set_irs_address(&e, &token, &irs_id);
        arm_hooks(&e);
    });

    client.set_time_transfer_limit(&token, &Limit { limit_time: 60, limit_value: 100 });
    client.pre_set_transfer_counter(
        &token,
        &sender_identity,
        &60,
        &TransferCounter { value: 90, timer: e.ledger().timestamp().saturating_add(60) },
    );

    assert!(!client.can_transfer(&sender.clone(), &recipient.clone(), &11, &token));
    assert!(client.can_transfer(&sender, &recipient, &10, &token));
}

#[test]
#[should_panic(expected = "Error(Contract, #400)")]
fn set_time_transfer_limit_rejects_more_than_four_limits() {
    let e = Env::default();
    e.mock_all_auths();

    let module_id = e.register(TestTimeTransfersLimitsContract, ());
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let client = TestTimeTransfersLimitsContractClient::new(&e, &module_id);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);
    });

    for limit_time in [60_u64, 120, 180, 240] {
        client.set_time_transfer_limit(&token, &Limit { limit_time, limit_value: 100 });
    }

    client.set_time_transfer_limit(&token, &Limit { limit_time: 300, limit_value: 100 });
}
