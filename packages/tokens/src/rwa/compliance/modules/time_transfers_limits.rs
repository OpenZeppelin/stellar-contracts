//! Time-windowed transfer-limits compliance module — Stellar port of T-REX
//! [`TimeTransfersLimitsModule.sol`][trex-src].
//!
//! Limits transfer volume within configurable time windows, tracking counters
//! per **identity** (not per wallet) — matching the EVM module's
//! `usersCounters[compliance][identity]`.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                        |
//! |------------------------|-----------------|--------------------------------------------------|
//! | `moduleCheck`          | `can_transfer`  | Validate transfer against all configured windows |
//! | _(same)_               | `can_create`    | Always true (mints don't count toward limits)    |
//! | `moduleTransferAction` | `on_transfer`   | Resolve sender identity, increase counters       |
//! | `moduleMintAction`     | `on_created`    | No-op                                            |
//! | `moduleBurnAction`     | `on_destroyed`  | No-op                                            |
//!
//! ## Differences from T-REX
//!
//! - T-REX `moduleCheck` returns true for token agents (`_isTokenAgent`).
//!   In Stellar, agent permissions are handled by the token's RBAC layer
//!   before compliance hooks fire, so the bypass is not replicated here.
//! - Limits and counters are token-scoped. Window reset behavior is explicit
//!   and deterministic (`timer <= now` starts a fresh bucket).
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeTransfersLimitsModule.sol

use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, panic_with_error, Address, Env, Vec,
};

use crate::rwa::compliance::ComplianceModule;

use super::common::{
    checked_add_i128, get_compliance_address, get_irs_client, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address, set_irs_address,
    ModuleError,
};

const MAX_LIMITS_PER_TOKEN: u32 = 4;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Limit {
    pub limit_time: u64,
    pub limit_value: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferCounter {
    pub value: i128,
    pub timer: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Limits(Address),
    /// Counter keyed by (token, identity, window_seconds).
    Counter(Address, Address, u64),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitUpdated {
    #[topic]
    pub token: Address,
    pub limit: Limit,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitRemoved {
    #[topic]
    pub token: Address,
    pub limit_time: u64,
}

#[contract]
pub struct TimeTransfersLimitsModule;

#[contractimpl]
impl TimeTransfersLimitsModule {
    /// Configures the IRS address used for identity lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    pub fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit) {
        require_compliance_auth(e);
        require_non_negative_amount(e, limit.limit_value);
        let mut limits: Vec<Limit> = e
            .storage()
            .persistent()
            .get(&DataKey::Limits(token.clone()))
            .unwrap_or_else(|| Vec::new(e));

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
                panic_with_error!(e, ModuleError::MathOverflow);
            }
            limits.push_back(limit.clone());
        }

        e.storage()
            .persistent()
            .set(&DataKey::Limits(token.clone()), &limits);
        TimeTransferLimitUpdated { token, limit }.publish(e);
    }

    pub fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>) {
        require_compliance_auth(e);
        for limit in limits.iter() {
            Self::set_time_transfer_limit(e, token.clone(), limit);
        }
    }

    pub fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64) {
        require_compliance_auth(e);
        let mut limits: Vec<Limit> = e
            .storage()
            .persistent()
            .get(&DataKey::Limits(token.clone()))
            .unwrap_or_else(|| Vec::new(e));

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
            panic_with_error!(e, ModuleError::MissingLimit);
        }

        e.storage()
            .persistent()
            .set(&DataKey::Limits(token.clone()), &limits);
        TimeTransferLimitRemoved { token, limit_time }.publish(e);
    }

    /// Removes multiple time transfer limits in a single call.
    /// Mirrors T-REX `batchRemoveTimeTransferLimit`.
    pub fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>) {
        require_compliance_auth(e);
        for lt in limit_times.iter() {
            Self::remove_time_transfer_limit(e, token.clone(), lt);
        }
    }

    pub fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<Limit> {
        e.storage()
            .persistent()
            .get(&DataKey::Limits(token))
            .unwrap_or_else(|| Vec::new(e))
    }

    fn is_counter_finished(e: &Env, token: &Address, identity: &Address, limit_time: u64) -> bool {
        let counter: TransferCounter = e
            .storage()
            .persistent()
            .get(&DataKey::Counter(token.clone(), identity.clone(), limit_time))
            .unwrap_or(TransferCounter { value: 0, timer: 0 });
        counter.timer <= e.ledger().timestamp()
    }

    fn reset_counter_if_needed(e: &Env, token: &Address, identity: &Address, limit_time: u64) {
        if Self::is_counter_finished(e, token, identity, limit_time) {
            let counter = TransferCounter {
                value: 0,
                timer: e.ledger().timestamp().saturating_add(limit_time),
            };
            e.storage()
                .persistent()
                .set(&DataKey::Counter(token.clone(), identity.clone(), limit_time), &counter);
        }
    }

    fn increase_counters(e: &Env, token: &Address, identity: &Address, value: i128) {
        let limits = Self::get_time_transfer_limits(e, token.clone());
        for limit in limits.iter() {
            Self::reset_counter_if_needed(e, token, identity, limit.limit_time);
            let key = DataKey::Counter(token.clone(), identity.clone(), limit.limit_time);
            let mut counter: TransferCounter = e
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or(TransferCounter { value: 0, timer: 0 });
            counter.value = checked_add_i128(e, counter.value, value);
            e.storage().persistent().set(&key, &counter);
        }
    }
}

#[contractimpl]
impl ComplianceModule for TimeTransfersLimitsModule {
    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        Self::increase_counters(e, &token, &from_id, amount);
    }

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// T-REX `moduleCheck` also bypasses limits for token agents via
    /// `_isTokenAgent`. In Stellar, agent-level permissions are handled by
    /// the token contract's RBAC layer before compliance hooks fire, so
    /// the bypass is not replicated here.
    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        if amount < 0 {
            return false;
        }
        let irs = get_irs_client(e, &token);
        let from_id = irs.stored_identity(&from);
        let limits = Self::get_time_transfer_limits(e, token.clone());

        for limit in limits.iter() {
            if amount > limit.limit_value {
                return false;
            }

            if !Self::is_counter_finished(e, &token, &from_id, limit.limit_time) {
                let counter: TransferCounter = e
                    .storage()
                    .persistent()
                    .get(&DataKey::Counter(token.clone(), from_id.clone(), limit.limit_time))
                    .unwrap_or(TransferCounter { value: 0, timer: 0 });
                if checked_add_i128(e, counter.value, amount) > limit.limit_value {
                    return false;
                }
            }
        }

        true
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "TimeTransfersLimitsModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}

#[cfg(test)]
mod test {
    use soroban_sdk::{contract, testutils::Address as _, Address, Env};

    use crate::rwa::compliance::ComplianceModuleClient;

    use super::{Limit, TimeTransfersLimitsModule};
    use crate::rwa::compliance::modules::test_utils::{MockIRS, MockIRSClient};

    #[contract]
    struct MockCompliance;

    #[test]
    fn transfer_limit_checks_accumulated_window() {
        let e = Env::default();
        e.mock_all_auths();
        let module = e.register(TimeTransfersLimitsModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);
        let module_client = super::TimeTransfersLimitsModuleClient::new(&e, &module);

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.set_time_transfer_limit(
                &token,
                &Limit {
                    limit_time: 3_600,
                    limit_value: 100,
                },
            );
        });

        assert!(client.can_transfer(&from, &to, &60, &token));
        e.as_contract(&compliance, || {
            client.on_transfer(&from, &to, &60, &token);
        });
        assert!(!client.can_transfer(&from, &to, &50, &token));
    }

    #[test]
    fn shared_identity_shares_counters() {
        let e = Env::default();
        e.mock_all_auths();
        let module = e.register(TimeTransfersLimitsModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());

        let wallet_a = Address::generate(&e);
        let wallet_b = Address::generate(&e);
        let shared_id = Address::generate(&e);
        let to = Address::generate(&e);

        let irs_helper = MockIRSClient::new(&e, &irs);
        irs_helper.mock_set_identity(&wallet_a, &shared_id);
        irs_helper.mock_set_identity(&wallet_b, &shared_id);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);
        let module_client = super::TimeTransfersLimitsModuleClient::new(&e, &module);

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.set_time_transfer_limit(
                &token,
                &Limit {
                    limit_time: 3_600,
                    limit_value: 100,
                },
            );
        });

        e.as_contract(&compliance, || {
            client.on_transfer(&wallet_a, &to, &70, &token);
        });
        // wallet_b shares identity with wallet_a, counter already at 70
        assert!(!client.can_transfer(&wallet_b, &to, &40, &token));
        assert!(client.can_transfer(&wallet_b, &to, &30, &token));
    }
}
