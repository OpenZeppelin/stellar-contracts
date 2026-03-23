#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        initial_lockup_period::{
            storage::{
                get_internal_balance, get_locks, get_lockup_period, get_total_locked,
                set_internal_balance, set_locks, set_lockup_period, set_total_locked,
            },
            InitialLockupPeriod, LockedTokens, LockupPeriodSet,
        },
        storage::{
            add_i128_or_panic, set_compliance_address, sub_i128_or_panic, verify_required_hooks,
            ComplianceModuleStorageKey,
        },
    },
    ComplianceHook,
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct InitialLockupPeriodContract;

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

fn calculate_unlocked_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let now = e.ledger().timestamp();
    let mut unlocked = 0i128;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if lock.release_timestamp <= now {
            unlocked = add_i128_or_panic(e, unlocked, lock.amount);
        }
    }
    unlocked
}

fn update_locked_tokens(e: &Env, token: &Address, wallet: &Address, mut amount_to_consume: i128) {
    let locks = get_locks(e, token, wallet);
    let now = e.ledger().timestamp();
    let mut new_locks = Vec::new(e);
    let mut consumed_total = 0i128;

    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if amount_to_consume > 0 && lock.release_timestamp <= now {
            if amount_to_consume >= lock.amount {
                amount_to_consume = sub_i128_or_panic(e, amount_to_consume, lock.amount);
                consumed_total = add_i128_or_panic(e, consumed_total, lock.amount);
            } else {
                consumed_total = add_i128_or_panic(e, consumed_total, amount_to_consume);
                new_locks.push_back(LockedTokens {
                    amount: sub_i128_or_panic(e, lock.amount, amount_to_consume),
                    release_timestamp: lock.release_timestamp,
                });
                amount_to_consume = 0;
            }
        } else {
            new_locks.push_back(lock);
        }
    }

    set_locks(e, token, wallet, &new_locks);

    let total_locked = get_total_locked(e, token, wallet);
    set_total_locked(e, token, wallet, sub_i128_or_panic(e, total_locked, consumed_total));
}

#[contractimpl]
impl InitialLockupPeriodContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl InitialLockupPeriod for InitialLockupPeriodContract {
    fn set_lockup_period(e: &Env, token: Address, lockup_seconds: u64) {
        require_module_admin_or_compliance_auth(e);
        set_lockup_period(e, &token, lockup_seconds);
        LockupPeriodSet { token, lockup_seconds }.publish(e);
    }

    fn pre_set_lockup_state(
        e: &Env,
        token: Address,
        wallet: Address,
        balance: i128,
        locks: Vec<LockedTokens>,
    ) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, balance);

        let mut total_locked = 0i128;
        for i in 0..locks.len() {
            let lock = locks.get(i).unwrap();
            stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(
                e,
                lock.amount,
            );
            total_locked = add_i128_or_panic(e, total_locked, lock.amount);
        }

        assert!(
            total_locked <= balance,
            "InitialLockupPeriodModule: total locked amount cannot exceed balance"
        );

        set_internal_balance(e, &token, &wallet, balance);
        set_locks(e, &token, &wallet, &locks);
        set_total_locked(e, &token, &wallet, total_locked);
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![
            e,
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);

        let total_locked = get_total_locked(e, &token, &from);

        if total_locked > 0 {
            let pre_balance = get_internal_balance(e, &token, &from);
            let pre_free = pre_balance - total_locked;

            if amount > pre_free.max(0) {
                let to_consume = amount - pre_free.max(0);
                update_locked_tokens(e, &token, &from, to_consume);
            }
        }

        let from_bal = get_internal_balance(e, &token, &from);
        set_internal_balance(e, &token, &from, sub_i128_or_panic(e, from_bal, amount));

        let to_bal = get_internal_balance(e, &token, &to);
        set_internal_balance(e, &token, &to, add_i128_or_panic(e, to_bal, amount));
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);

        let period = get_lockup_period(e, &token);
        if period > 0 {
            let mut locks = get_locks(e, &token, &to);
            locks.push_back(LockedTokens {
                amount,
                release_timestamp: e.ledger().timestamp().saturating_add(period),
            });
            set_locks(e, &token, &to, &locks);

            let total = get_total_locked(e, &token, &to);
            set_total_locked(e, &token, &to, add_i128_or_panic(e, total, amount));
        }

        let current = get_internal_balance(e, &token, &to);
        set_internal_balance(e, &token, &to, add_i128_or_panic(e, current, amount));
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);

        let total_locked = get_total_locked(e, &token, &from);

        if total_locked > 0 {
            let pre_balance = get_internal_balance(e, &token, &from);
            let mut free_amount = pre_balance - total_locked;

            if free_amount < amount {
                let locks = get_locks(e, &token, &from);
                free_amount += calculate_unlocked_amount(e, &locks);
            }

            assert!(
                free_amount >= amount,
                "InitialLockupPeriodModule: insufficient unlocked balance for burn"
            );

            let pre_free = pre_balance - total_locked;
            if amount > pre_free.max(0) {
                let to_consume = amount - pre_free.max(0);
                update_locked_tokens(e, &token, &from, to_consume);
            }
        }

        let current = get_internal_balance(e, &token, &from);
        set_internal_balance(e, &token, &from, sub_i128_or_panic(e, current, amount));
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
