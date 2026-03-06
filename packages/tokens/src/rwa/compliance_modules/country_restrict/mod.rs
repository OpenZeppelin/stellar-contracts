//! Country restriction compliance module — Stellar port of T-REX
//! [`CountryRestrictModule.sol`][trex-src].
//!
//! Recipients whose identity has a country code on the restriction list are
//! blocked from receiving tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryRestrictModule.sol

pub mod storage;

use soroban_sdk::{contractevent, contracttrait, Address, Env, String, Vec};
use storage::{is_country_restricted, remove_country_restricted, set_country_restricted};

use super::common::{
    country_code, get_compliance_address, get_irs_client, module_name, require_compliance_auth,
    set_compliance_address, set_irs_address,
};

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

/// Country restriction compliance trait.
///
/// Provides default implementations for maintaining a per-token country
/// restriction list and blocking transfers/mints to recipients from
/// restricted countries via the Identity Registry Storage.
#[contracttrait]
pub trait CountryRestrict {
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
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    /// Adds a country to the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `country` - The ISO 3166-1 numeric country code to restrict.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryRestricted`].
    fn add_country_restriction(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        set_country_restricted(e, &token, country);
        CountryRestricted { token, country }.publish(e);
    }

    /// Removes a country from the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `country` - The ISO 3166-1 numeric country code to unrestrict.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryUnrestricted`].
    fn remove_country_restriction(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        remove_country_restricted(e, &token, country);
        CountryUnrestricted { token, country }.publish(e);
    }

    /// Adds multiple countries to the restriction list in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `countries` - The country codes to restrict.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryRestricted`] for each country added.
    fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            set_country_restricted(e, &token, country);
            CountryRestricted { token: token.clone(), country }.publish(e);
        }
    }

    /// Removes multiple countries from the restriction list in a single
    /// call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `countries` - The country codes to unrestrict.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`CountryUnrestricted`] for each country removed.
    fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            remove_country_restricted(e, &token, country);
            CountryUnrestricted { token: token.clone(), country }.publish(e);
        }
    }

    /// Returns whether `country` is on the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `country` - The ISO 3166-1 numeric country code.
    fn is_country_restricted(e: &Env, token: Address, country: u32) -> bool {
        is_country_restricted(e, &token, country)
    }

    /// No-op — this module does not track transfer state.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — this module does not track mint state.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — this module does not track burn state.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Checks whether `to` has any restricted country in the IRS.
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
    /// `false` if the recipient has any restricted country, `true`
    /// otherwise.
    ///
    /// # Cross-Contract Calls
    ///
    /// Calls the IRS to resolve country data for `to`.
    fn can_transfer(e: &Env, _from: Address, to: Address, _amount: i128, token: Address) -> bool {
        let irs = get_irs_client(e, &token);
        let entries = irs.get_country_data_entries(&to);
        for entry in entries.iter() {
            if is_country_restricted(e, &token, country_code(&entry.country)) {
                return false;
            }
        }
        true
    }

    /// Delegates to [`can_transfer`](CountryRestrict::can_transfer) — same
    /// country check applies to mints.
    fn can_create(e: &Env, to: Address, _amount: i128, token: Address) -> bool {
        Self::can_transfer(e, to.clone(), to, 0, token)
    }

    /// Returns the module name for identification.
    fn name(e: &Env) -> String {
        module_name(e, "CountryRestrictModule")
    }

    /// Returns the compliance contract address.
    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    /// Sets the compliance contract address (one-time only).
    ///
    /// # Panics
    ///
    /// Panics if the compliance address has already been set.
    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
