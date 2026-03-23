#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, vec, Address, Env, String, Vec,
};
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{
            add_i128_or_panic, get_irs_client, set_compliance_address, set_irs_address,
            verify_required_hooks, ComplianceModuleStorageKey,
        },
        time_transfers_limits::{
            storage::{get_counter, get_limits, set_counter, set_limits},
            Limit, TimeTransferLimitRemoved, TimeTransferLimitUpdated, TimeTransfersLimits,
            TransferCounter,
        },
        ComplianceModuleError,
    },
    ComplianceHook,
};

const MAX_LIMITS_PER_TOKEN: u32 = 4;

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct TimeTransfersLimitsContract;

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

fn is_counter_finished(e: &Env, token: &Address, identity: &Address, limit_time: u64) -> bool {
    let counter = get_counter(e, token, identity, limit_time);
    counter.timer <= e.ledger().timestamp()
}

fn reset_counter_if_needed(e: &Env, token: &Address, identity: &Address, limit_time: u64) {
    if is_counter_finished(e, token, identity, limit_time) {
        let counter =
            TransferCounter { value: 0, timer: e.ledger().timestamp().saturating_add(limit_time) };
        set_counter(e, token, identity, limit_time, &counter);
    }
}

fn increase_counters(e: &Env, token: &Address, identity: &Address, value: i128) {
    let limits = get_limits(e, token);
    for limit in limits.iter() {
        reset_counter_if_needed(e, token, identity, limit.limit_time);
        let mut counter = get_counter(e, token, identity, limit.limit_time);
        counter.value = add_i128_or_panic(e, counter.value, value);
        set_counter(e, token, identity, limit.limit_time, &counter);
    }
}

#[contractimpl]
impl TimeTransfersLimitsContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl TimeTransfersLimits for TimeTransfersLimitsContract {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit) {
        require_module_admin_or_compliance_auth(e);
        assert!(limit.limit_time > 0, "limit_time must be greater than zero");
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(
            e,
            limit.limit_value,
        );
        let mut limits = get_limits(e, &token);

        let mut replaced = false;
        for i in 0..limits.len() {
            let current = limits.get(i).expect("limit exists");
            if current.limit_time == limit.limit_time {
                limits.set(i, limit.clone());
                replaced = true;
                break;
            }
        }

        if !replaced {
            if limits.len() >= MAX_LIMITS_PER_TOKEN {
                panic_with_error!(e, ComplianceModuleError::TooManyLimits);
            }
            limits.push_back(limit.clone());
        }

        set_limits(e, &token, &limits);
        TimeTransferLimitUpdated { token, limit }.publish(e);
    }

    fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>) {
        require_module_admin_or_compliance_auth(e);
        for limit in limits.iter() {
            Self::set_time_transfer_limit(e, token.clone(), limit);
        }
    }

    fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64) {
        require_module_admin_or_compliance_auth(e);
        let mut limits = get_limits(e, &token);

        let mut found = false;
        for i in 0..limits.len() {
            let current = limits.get(i).expect("limit exists");
            if current.limit_time == limit_time {
                limits.remove(i);
                found = true;
                break;
            }
        }

        if !found {
            panic_with_error!(e, ComplianceModuleError::MissingLimit);
        }

        set_limits(e, &token, &limits);
        TimeTransferLimitRemoved { token, limit_time }.publish(e);
    }

    fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>) {
        require_module_admin_or_compliance_auth(e);
        for lt in limit_times.iter() {
            Self::remove_time_transfer_limit(e, token.clone(), lt);
        }
    }

    fn pre_set_transfer_counter(
        e: &Env,
        token: Address,
        identity: Address,
        limit_time: u64,
        counter: TransferCounter,
    ) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(
            e,
            counter.value,
        );
        assert!(limit_time > 0, "limit_time must be greater than zero");

        let mut found = false;
        for limit in get_limits(e, &token).iter() {
            if limit.limit_time == limit_time {
                found = true;
                break;
            }
        }

        if !found {
            panic_with_error!(e, ComplianceModuleError::MissingLimit);
        }

        set_counter(e, &token, &identity, limit_time, &counter);
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![e, ComplianceHook::CanTransfer, ComplianceHook::Transferred]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);
        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        increase_counters(e, &token, &from_id, amount);
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
