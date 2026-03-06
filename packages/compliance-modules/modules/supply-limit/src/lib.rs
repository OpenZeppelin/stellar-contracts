#![no_std]
//! Supply cap compliance module — Stellar port of T-REX
//! [`SupplyLimitModule.sol`][trex-src].
//!
//! Caps the total number of tokens that can be minted for a given token.
//! Regular transfers are always allowed.
//!
//! ## Internal state tracking
//!
//! Instead of calling `token.total_supply()` (which would cause a forbidden
//! re-entrant call on Soroban), this module maintains its own internal supply
//! counter. The counter is updated via the `on_created` and `on_destroyed`
//! hooks, so the module **must** be wired to `Created` and `Destroyed` hooks
//! in addition to `CanCreate`.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                      |
//! |------------------------|-----------------|-------------------------------------------------|
//! | `moduleCheck`          | `can_create`    | Enforce `internalSupply + amount ≤ limit`      |
//! | _(same)_               | `can_transfer`  | Always true (transfers don't affect supply)    |
//! | `moduleTransferAction` | `on_transfer`   | No-op                                          |
//! | `moduleMintAction`     | `on_created`    | `internalSupply += amount`                     |
//! | `moduleBurnAction`     | `on_destroyed`  | `internalSupply -= amount`                     |
//!
//! ## Required hooks
//!
//! `CanCreate`, `Created`, `Destroyed`
//!
//! Call `verify_hook_wiring()` after wiring to arm the module. The `can_create`
//! hook panics if the module is not armed — this prevents silent
//! misconfiguration where missing hooks would cause the internal supply counter
//! to drift.
//!
//! ## Differences from T-REX
//!
//! - A zero cap is treated as "not configured" (mints pass). T-REX blocks mints
//!   when the limit is zero because `totalSupply + value > 0` is always true.
//!   Our interpretation aligns with plug-and-play semantics: adding the module
//!   without configuring a limit should not block operations.
//! - Uses internal supply counter instead of `token.totalSupply()` to avoid
//!   Soroban's contract re-entry restriction.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/SupplyLimitModule.sol

pub mod storage;

use soroban_sdk::{contract, contractevent, contractimpl, vec, Address, Env, Vec};
use stellar_compliance_common::{
    checked_add_i128, checked_sub_i128, get_compliance_address, hooks_verified, module_name,
    require_compliance_auth, require_non_negative_amount, set_compliance_address,
    verify_required_hooks,
};
use stellar_tokens::rwa::compliance::{ComplianceHook, ComplianceModule};

use storage::{
    get_internal_supply, get_supply_limit, get_supply_limit_or_panic, set_internal_supply,
    set_supply_limit,
};

/// Emitted when a token's supply cap is configured or changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyLimitSet {
    #[topic]
    pub token: Address,
    pub limit: i128,
}

/// Enforces a global supply ceiling per token.
#[contract]
pub struct SupplyLimitModule;

#[contractimpl]
impl SupplyLimitModule {
    /// Configures the maximum supply cap for `token`.
    /// T-REX equivalent: `setSupplyLimit(_newLimit)`.
    pub fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, limit);
        set_supply_limit(e, &token, limit);
        SupplyLimitSet { token, limit }.publish(e);
    }

    /// Returns the configured supply limit for `token`.
    pub fn get_supply_limit(e: &Env, token: Address) -> i128 {
        get_supply_limit_or_panic(e, &token)
    }

    /// Returns the module's internal supply counter for `token`.
    pub fn get_internal_supply(e: &Env, token: Address) -> i128 {
        get_internal_supply(e, &token)
    }

    /// Returns the compliance hooks this module must be registered on.
    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![e, ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed]
    }

    /// Arms the module by verifying all required hooks are wired.
    ///
    /// Must be called **once after wiring** (outside the hook chain) because
    /// it cross-calls the compliance contract. Panics with a message naming
    /// the first missing hook. Caches the result so that subsequent `can_*`
    /// calls only check a boolean flag.
    pub fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }
}

#[contractimpl]
impl ComplianceModule for SupplyLimitModule {
    /// No-op — transfers don't affect total supply.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// Increments the internal supply counter on mint.
    fn on_created(e: &Env, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let current = get_internal_supply(e, &token);
        set_internal_supply(e, &token, checked_add_i128(e, current, amount));
    }

    /// Decrements the internal supply counter on burn.
    fn on_destroyed(e: &Env, _from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        require_non_negative_amount(e, amount);
        let current = get_internal_supply(e, &token);
        set_internal_supply(e, &token, checked_sub_i128(e, current, amount));
    }

    /// Always returns `true` — supply limit only gates minting.
    fn can_transfer(
        _e: &Env,
        _from: Address,
        _to: Address,
        _amount: i128,
        _token: Address,
    ) -> bool {
        true
    }

    /// Returns `true` if `internal_supply + amount` is within the cap.
    fn can_create(e: &Env, _to: Address, amount: i128, token: Address) -> bool {
        assert!(
            hooks_verified(e),
            "SupplyLimitModule: not armed — call verify_hook_wiring() after wiring hooks \
             [CanCreate, Created, Destroyed]"
        );
        if amount < 0 {
            return false;
        }
        let limit = get_supply_limit(e, &token);
        if limit == 0 {
            return true;
        }
        let supply = get_internal_supply(e, &token);
        checked_add_i128(e, supply, amount) <= limit
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "SupplyLimitModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
