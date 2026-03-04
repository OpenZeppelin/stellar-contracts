//! Supply cap compliance module — Stellar port of T-REX
//! [`SupplyLimitModule.sol`][trex-src].
//!
//! Caps the total number of tokens that can be minted for a given token.
//! Regular transfers are always allowed.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                      |
//! |------------------------|-----------------|-------------------------------------------------|
//! | `moduleCheck`          | `can_create`    | Enforce `totalSupply + amount ≤ limit` on mint |
//! | _(same)_               | `can_transfer`  | Always true (transfers don't affect supply)    |
//! | `moduleTransferAction` | `on_transfer`   | No-op                                          |
//! | `moduleMintAction`     | `on_created`    | No-op                                          |
//! | `moduleBurnAction`     | `on_destroyed`  | No-op                                          |
//!
//! ## Differences from T-REX
//!
//! - A zero cap is treated as "not configured" (mints pass). T-REX blocks
//!   mints when the limit is zero because `totalSupply + value > 0` is always
//!   true. Our interpretation aligns with plug-and-play semantics: adding the
//!   module without configuring a limit should not block operations.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/SupplyLimitModule.sol

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, panic_with_error, Address, Env};

use crate::rwa::compliance::ComplianceModule;

use super::common::{
    get_compliance_address, module_name, require_compliance_auth, require_non_negative_amount,
    set_compliance_address, checked_add_i128, ModuleError, TokenSupplyViewClient,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    // Token-scoped cap to preserve multi-token compatibility.
    SupplyLimit(Address),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyLimitSet {
    #[topic]
    pub token: Address,
    pub limit: i128,
}

#[contract]
pub struct SupplyLimitModule;

#[contractimpl]
impl SupplyLimitModule {
    pub fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        require_compliance_auth(e);
        require_non_negative_amount(e, limit);
        e.storage()
            .persistent()
            .set(&DataKey::SupplyLimit(token.clone()), &limit);
        SupplyLimitSet { token, limit }.publish(e);
    }

    pub fn get_supply_limit(e: &Env, token: Address) -> i128 {
        e.storage()
            .persistent()
            .get(&DataKey::SupplyLimit(token))
            .unwrap_or_else(|| panic_with_error!(e, ModuleError::MissingLimit))
    }
}

#[contractimpl]
impl ComplianceModule for SupplyLimitModule {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn can_create(e: &Env, _to: Address, amount: i128, token: Address) -> bool {
        if amount < 0 {
            return false;
        }
        let limit: i128 = e
            .storage()
            .persistent()
            .get(&DataKey::SupplyLimit(token.clone()))
            .unwrap_or_default();
        if limit == 0 {
            // Match T-REX style behavior: zero means "no configured cap".
            return true;
        }
        let total_supply = TokenSupplyViewClient::new(e, &token).total_supply();
        // Overflow-safe sum to avoid silently wrapping total supply checks.
        checked_add_i128(e, total_supply, amount) <= limit
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

#[cfg(test)]
mod test {
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env};

    use crate::rwa::compliance::ComplianceModuleClient;

    use super::SupplyLimitModule;

    #[contract]
    struct MockToken;

    #[contract]
    struct MockCompliance;

    #[contractimpl]
    impl MockToken {
        pub fn total_supply(_e: &Env) -> i128 {
            100
        }
    }

    #[test]
    fn supply_limit_rejects_over_limit_mint() {
        let e = Env::default();
        e.mock_all_auths();

        let module = e.register(SupplyLimitModule, ());
        let token = e.register(MockToken, ());
        let compliance = e.register(MockCompliance, ());
        let to = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);

        let module_client = super::SupplyLimitModuleClient::new(&e, &module);
        e.as_contract(&compliance, || {
            module_client.set_supply_limit(&token, &120);
        });

        assert!(!client.can_create(&to, &21, &token));
        assert!(client.can_create(&to, &20, &token));
    }
}
