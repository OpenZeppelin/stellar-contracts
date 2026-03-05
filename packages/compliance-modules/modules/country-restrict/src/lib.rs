#![no_std]
//! Country restriction compliance module — Stellar port of T-REX
//! [`CountryRestrictModule.sol`][trex-src].
//!
//! Recipients whose identity has a country code on the restriction list are
//! blocked from receiving tokens.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                      |
//! |------------------------|-----------------|-------------------------------------------------|
//! | `moduleCheck`          | `can_transfer`  | Resolve recipient country, check restriction   |
//! | _(same)_               | `can_create`    | Delegates to `can_transfer`                    |
//! | `moduleTransferAction` | `on_transfer`   | No-op                                          |
//! | `moduleMintAction`     | `on_created`    | No-op                                          |
//! | `moduleBurnAction`     | `on_destroyed`  | No-op                                          |
//!
//! ## Identity Resolution
//!
//! Semantics are the inverse of `country_allow`. Because an investor can have
//! multiple country entries, the check **blocks** if **any** country code is
//! restricted — the most conservative interpretation.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryRestrictModule.sol

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env, Vec};

use stellar_tokens::rwa::compliance::ComplianceModule;

use stellar_compliance_common::{
    country_code, get_compliance_address, get_irs_client, module_name, require_compliance_auth,
    set_compliance_address, set_irs_address,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Per-(token, country) restriction flag.
    RestrictedCountry(Address, u32),
}

/// Emitted when a country is added to the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryRestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emitted when a country is removed from the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnrestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Blocks token transfers to recipients from restricted countries.
#[contract]
pub struct CountryRestrictModule;

#[contractimpl]
impl CountryRestrictModule {
    /// Configures the IRS address used for country lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    /// Adds a country code to the restriction list for `token`.
    pub fn add_country_restriction(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        e.storage().persistent().set(&DataKey::RestrictedCountry(token.clone(), country), &true);
        CountryRestricted { token, country }.publish(e);
    }

    /// Removes a country code from the restriction list for `token`.
    pub fn remove_country_restriction(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        e.storage().persistent().remove(&DataKey::RestrictedCountry(token.clone(), country));
        CountryUnrestricted { token, country }.publish(e);
    }

    /// Adds multiple country codes to the restriction list in a single call.
    pub fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            e.storage()
                .persistent()
                .set(&DataKey::RestrictedCountry(token.clone(), country), &true);
            CountryRestricted { token: token.clone(), country }.publish(e);
        }
    }

    /// Removes multiple country codes from the restriction list in a single call.
    pub fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            e.storage().persistent().remove(&DataKey::RestrictedCountry(token.clone(), country));
            CountryUnrestricted { token: token.clone(), country }.publish(e);
        }
    }

    /// Returns `true` if `country` is on the restriction list for `token`.
    pub fn is_country_restricted(e: &Env, token: Address, country: u32) -> bool {
        e.storage()
            .persistent()
            .get(&DataKey::RestrictedCountry(token, country))
            .unwrap_or_default()
    }
}

#[contractimpl]
impl ComplianceModule for CountryRestrictModule {
    /// No-op — stateless module.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — stateless module.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — stateless module.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Returns `false` if any of the recipient's country codes is restricted.
    fn can_transfer(e: &Env, _from: Address, to: Address, _amount: i128, token: Address) -> bool {
        let irs = get_irs_client(e, &token);
        let entries = irs.get_country_data_entries(&to);
        for entry in entries.iter() {
            if Self::is_country_restricted(e, token.clone(), country_code(&entry.country)) {
                return false;
            }
        }
        true
    }

    /// Delegates to `can_transfer` — mints are subject to the same check.
    fn can_create(e: &Env, to: Address, _amount: i128, token: Address) -> bool {
        Self::can_transfer(e, to.clone(), to, 0, token)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "CountryRestrictModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
