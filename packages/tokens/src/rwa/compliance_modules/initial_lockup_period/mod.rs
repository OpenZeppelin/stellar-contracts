//! Initial lockup period compliance module — Stellar port of T-REX
//! [`TimeExchangeLimitsModule.sol`][trex-src].
//!
//! Enforces a lockup period for all investors whenever they receive tokens
//! through primary emissions (mints). Tokens received via peer-to-peer
//! transfers are **not** subject to lockup restrictions.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeExchangeLimitsModule.sol

pub mod storage;

use soroban_sdk::{contractevent, contracttrait, vec, Address, Env, String, Vec};

use super::common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, hooks_verified, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address,
    verify_required_hooks,
};
use crate::rwa::compliance::ComplianceHook;
pub use storage::LockedTokens;
use storage::{
    get_internal_balance, get_lockup_period, get_locks, get_total_locked, set_internal_balance,
    set_lockup_period, set_locks, set_total_locked,
};

/// Emitted when a token's lockup duration is configured or changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub lockup_seconds: u64,
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn calculate_unlocked_amount(e: &Env, locks: &Vec<LockedTokens>) -> i128 {
    let now = e.ledger().timestamp();
    let mut unlocked = 0i128;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if lock.release_timestamp <= now {
            unlocked = checked_add_i128(e, unlocked, lock.amount);
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
                amount_to_consume -= lock.amount;
                consumed_total += lock.amount;
            } else {
                consumed_total += amount_to_consume;
                new_locks.push_back(LockedTokens {
                    amount: lock.amount - amount_to_consume,
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
    set_total_locked(e, token, wallet, checked_sub_i128(e, total_locked, consumed_total));
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[contracttrait]
pub trait InitialLockupPeriod {
    fn set_lockup_period(e: &Env, token: Address, lockup_seconds: u64) {
        require_compliance_auth(e);
        set_lockup_period(e, &token, lockup_seconds);
        LockupPeriodSet { token, lockup_seconds }.publish(e);
    }

    fn get_lockup_period(e: &Env, token: Address) -> u64 {
        get_lockup_period(e, &token)
    }

    fn get_total_locked(e: &Env, token: Address, wallet: Address) -> i128 {
        get_total_locked(e, &token, &wallet)
    }

    fn get_locked_tokens(e: &Env, token: Address, wallet: Address) -> Vec<LockedTokens> {
        get_locks(e, &token, &wallet)
    }

    fn get_internal_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        get_internal_balance(e, &token, &wallet)
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
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

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
        set_internal_balance(e, &token, &from, checked_sub_i128(e, from_bal, amount));

        let to_bal = get_internal_balance(e, &token, &to);
        set_internal_balance(e, &token, &to, checked_add_i128(e, to_bal, amount));
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

        let period = get_lockup_period(e, &token);
        if period > 0 {
            let mut locks = get_locks(e, &token, &to);
            locks.push_back(LockedTokens {
                amount,
                release_timestamp: e.ledger().timestamp().saturating_add(period),
            });
            set_locks(e, &token, &to, &locks);

            let total = get_total_locked(e, &token, &to);
            set_total_locked(e, &token, &to, checked_add_i128(e, total, amount));
        }

        let current = get_internal_balance(e, &token, &to);
        set_internal_balance(e, &token, &to, checked_add_i128(e, current, amount));
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);

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
        set_internal_balance(e, &token, &from, checked_sub_i128(e, current, amount));
    }

    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        assert!(
            hooks_verified(e),
            "InitialLockupPeriodModule: not armed — call verify_hook_wiring() after wiring hooks \
             [CanTransfer, Created, Transferred, Destroyed]"
        );
        if amount < 0 {
            return false;
        }

        let total_locked = get_total_locked(e, &token, &from);
        if total_locked == 0 {
            return true;
        }

        let balance = get_internal_balance(e, &token, &from);
        let free = balance - total_locked;

        if free >= amount {
            return true;
        }

        let locks = get_locks(e, &token, &from);
        let unlocked = calculate_unlocked_amount(e, &locks);
        (free + unlocked) >= amount
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> String {
        module_name(e, "InitialLockupPeriodModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
