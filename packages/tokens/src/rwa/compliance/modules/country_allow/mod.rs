//! Country allowlist compliance module — Stellar port of T-REX
//! [`CountryAllowModule.sol`][trex-src].
//!
//! Only recipients whose identity has at least one country code in the
//! allowlist may receive tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryAllowModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, String, Vec};
use storage::{is_country_allowed, remove_country_allowed, set_country_allowed};

use super::storage::{
    country_code, get_compliance_address, get_irs_country_data_entries, module_name,
    set_irs_address,
};

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

/// Country allowlist compliance trait.
///
/// Provides default implementations for maintaining a per-token country
/// allowlist and validating transfers/mints against it via the Identity
/// Registry Storage.
#[contracttrait]
pub trait CountryAllow {
    /// Sets the Identity Registry Storage contract address for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token this IRS applies to.
    /// * `irs` - The IRS contract address.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        get_compliance_address(e).require_auth();
        set_irs_address(e, &token, &irs);
    }

    /// Adds a country to the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `country` - The ISO 3166-1 numeric country code to allow.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryAllowed`].
    fn add_allowed_country(e: &Env, token: Address, country: u32) {
        get_compliance_address(e).require_auth();
        set_country_allowed(e, &token, country);
        CountryAllowed { token, country }.publish(e);
    }

    /// Removes a country from the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `country` - The ISO 3166-1 numeric country code to remove.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryUnallowed`].
    fn remove_allowed_country(e: &Env, token: Address, country: u32) {
        get_compliance_address(e).require_auth();
        remove_country_allowed(e, &token, country);
        CountryUnallowed { token, country }.publish(e);
    }

    /// Adds multiple countries to the allowlist in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `countries` - The country codes to allow.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryAllowed`] for each country added.
    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        get_compliance_address(e).require_auth();
        for country in countries.iter() {
            set_country_allowed(e, &token, country);
            CountryAllowed { token: token.clone(), country }.publish(e);
        }
    }

    /// Removes multiple countries from the allowlist in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `countries` - The country codes to remove.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryUnallowed`] for each country removed.
    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        get_compliance_address(e).require_auth();
        for country in countries.iter() {
            remove_country_allowed(e, &token, country);
            CountryUnallowed { token: token.clone(), country }.publish(e);
        }
    }

    /// Returns whether `country` is on the allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `country` - The ISO 3166-1 numeric country code.
    fn is_country_allowed(e: &Env, token: Address, country: u32) -> bool {
        is_country_allowed(e, &token, country)
    }

    /// No-op — this module does not track transfer state.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — this module does not track mint state.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — this module does not track burn state.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Checks whether `to` has at least one allowed country in the IRS.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `_from` - The sender (unused).
    /// * `to` - The recipient whose country data is checked.
    /// * `_amount` - The transfer amount (unused).
    /// * `token` - The token address.
    ///
    /// # Returns
    ///
    /// `true` if the recipient has at least one allowed country, `false`
    /// otherwise.
    ///
    /// # Cross-Contract Calls
    ///
    /// Calls the IRS to resolve country data for `to`.
    fn can_transfer(e: &Env, _from: Address, to: Address, _amount: i128, token: Address) -> bool {
        let entries = get_irs_country_data_entries(e, &token, &to);
        for entry in entries.iter() {
            if is_country_allowed(e, &token, country_code(&entry.country)) {
                return true;
            }
        }
        false
    }

    /// Delegates to [`can_transfer`](CountryAllow::can_transfer) — same
    /// country check applies to mints.
    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        Self::can_transfer(e, to.clone(), to, amount, token)
    }

    /// Returns the module name for identification.
    fn name(e: &Env) -> String {
        module_name(e, "CountryAllowModule")
    }

    /// Returns the compliance contract address.
    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    /// Sets the compliance contract address (one-time only).
    ///
    /// Implementers must gate this entrypoint with bootstrap-admin auth before
    /// delegating to
    /// [`storage::set_compliance_address`](super::storage::set_compliance_address).
    ///
    ///
    /// # Panics
    ///
    /// Panics if the compliance address has already been set.
    fn set_compliance_address(e: &Env, compliance: Address);
}
