#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        max_balance::{
            storage::{get_id_balance, get_max_balance, set_id_balance, set_max_balance},
            IDBalancePreSet, MaxBalance, MaxBalanceSet,
        },
        storage::{
            add_i128_or_panic, get_irs_client, set_compliance_address, set_irs_address,
            sub_i128_or_panic, verify_required_hooks, ComplianceModuleStorageKey,
        },
    },
    ComplianceHook,
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct MaxBalanceContract;

fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).expect("admin must be set")
}

fn require_module_admin_or_compliance_auth(e: &Env) {
    if let Some(compliance) =
        e.storage().instance().get::<_, Address>(&ComplianceModuleStorageKey::Compliance)
    {
        compliance.require_auth();
    } else {
        get_admin(e).require_auth();
    }
}

#[contractimpl]
impl MaxBalanceContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl MaxBalance for MaxBalanceContract {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn set_max_balance(e: &Env, token: Address, max: i128) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, max);
        set_max_balance(e, &token, max);
        MaxBalanceSet { token, max_balance: max }.publish(e);
    }

    fn pre_set_module_state(e: &Env, token: Address, identity: Address, balance: i128) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, balance);
        set_id_balance(e, &token, &identity, balance);
        IDBalancePreSet { token, identity, balance }.publish(e);
    }

    fn batch_pre_set_module_state(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    ) {
        require_module_admin_or_compliance_auth(e);
        assert!(
            identities.len() == balances.len(),
            "MaxBalanceModule: identities and balances length mismatch"
        );
        for i in 0..identities.len() {
            let id = identities.get(i).unwrap();
            let bal = balances.get(i).unwrap();
            stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, bal);
            set_id_balance(e, &token, &id, bal);
            IDBalancePreSet { token: token.clone(), identity: id, balance: bal }.publish(e);
        }
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![
            e,
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        let to_id = irs.stored_identity(&to);

        if from_id == to_id {
            return;
        }

        let from_balance = get_id_balance(e, &token, &from_id);
        let to_balance = get_id_balance(e, &token, &to_id);
        let new_to_balance = add_i128_or_panic(e, to_balance, amount);

        let max = get_max_balance(e, &token);
        assert!(
            max == 0 || new_to_balance <= max,
            "MaxBalanceModule: recipient identity balance exceeds max"
        );

        set_id_balance(e, &token, &from_id, sub_i128_or_panic(e, from_balance, amount));
        set_id_balance(e, &token, &to_id, new_to_balance);
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let to_id = irs.stored_identity(&to);

        let current = get_id_balance(e, &token, &to_id);
        let new_balance = add_i128_or_panic(e, current, amount);

        let max = get_max_balance(e, &token);
        assert!(
            max == 0 || new_balance <= max,
            "MaxBalanceModule: recipient identity balance exceeds max after mint"
        );

        set_id_balance(e, &token, &to_id, new_balance);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);

        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);

        let current = get_id_balance(e, &token, &from_id);
        set_id_balance(e, &token, &from_id, sub_i128_or_panic(e, current, amount));
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
