#![no_std]
//! Country allowlist compliance module — Stellar port of T-REX
//! [`CountryAllowModule.sol`][trex-src].
//!
//! Only recipients whose identity has at least one country code in the
//! allowlist may receive tokens.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                    |
//! |------------------------|-----------------|----------------------------------------------|
//! | `moduleCheck`          | `can_transfer`  | Resolve recipient country, check allowlist   |
//! | _(same)_               | `can_create`    | Delegates to `can_transfer`                  |
//! | `moduleTransferAction` | `on_transfer`   | No-op                                        |
//! | `moduleMintAction`     | `on_created`    | No-op                                        |
//! | `moduleBurnAction`     | `on_destroyed`  | No-op                                        |
//!
//! ## Identity Resolution
//!
//! Country data is fetched cross-contract from the IRS at check time, matching
//! the T-REX `_getCountry(compliance, user)` pattern. Because Stellar's IRS
//! supports multiple country entries per investor (residence, citizenship, …),
//! the check passes if **any** of the investor's country codes is allowed.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryAllowModule.sol

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env, Vec};

use stellar_tokens::rwa::compliance::ComplianceModule;

use stellar_compliance_common::{
    country_code, get_compliance_address, get_irs_client, module_name, require_compliance_auth,
    set_compliance_address, set_irs_address,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Per-(token, country) allowlist flag.
    AllowedCountry(Address, u32),
}

/// Emitted when a country is added to the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryAllowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emitted when a country is removed from the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnallowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Only allows token transfers to recipients from approved countries.
#[contract]
pub struct CountryAllowModule;

#[contractimpl]
impl CountryAllowModule {
    /// Configures the IRS address used for country lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    /// Adds a country code to the allowlist for `token`.
    pub fn add_allowed_country(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        e.storage().persistent().set(&DataKey::AllowedCountry(token.clone(), country), &true);
        CountryAllowed { token, country }.publish(e);
    }

    /// Removes a country code from the allowlist for `token`.
    pub fn remove_allowed_country(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        e.storage()
            .persistent()
            .remove(&DataKey::AllowedCountry(token.clone(), country));
        CountryUnallowed { token, country }.publish(e);
    }

    /// Adds multiple country codes to the allowlist in a single call.
    pub fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            e.storage().persistent().set(&DataKey::AllowedCountry(token.clone(), country), &true);
            CountryAllowed { token: token.clone(), country }.publish(e);
        }
    }

    /// Removes multiple country codes from the allowlist in a single call.
    pub fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            e.storage()
                .persistent()
                .remove(&DataKey::AllowedCountry(token.clone(), country));
            CountryUnallowed { token: token.clone(), country }.publish(e);
        }
    }

    /// Returns `true` if `country` is on the allowlist for `token`.
    pub fn is_country_allowed(e: &Env, token: Address, country: u32) -> bool {
        e.storage().persistent().get(&DataKey::AllowedCountry(token, country)).unwrap_or_default()
    }
}

#[contractimpl]
impl ComplianceModule for CountryAllowModule {
    /// No-op — stateless module.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — stateless module.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — stateless module.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Returns `true` if any of the recipient's country codes is allowed.
    fn can_transfer(e: &Env, _from: Address, to: Address, _amount: i128, token: Address) -> bool {
        let irs = get_irs_client(e, &token);
        let entries = irs.get_country_data_entries(&to);
        for entry in entries.iter() {
            if Self::is_country_allowed(e, token.clone(), country_code(&entry.country)) {
                return true;
            }
        }
        false
    }

    /// Delegates to `can_transfer` — mints are subject to the same check.
    fn can_create(e: &Env, to: Address, _amount: i128, token: Address) -> bool {
        Self::can_transfer(e, to.clone(), to, 0, token)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "CountryAllowModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
