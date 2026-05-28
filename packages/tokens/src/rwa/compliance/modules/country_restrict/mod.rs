//! Country restriction compliance module — Stellar port of T-REX
//! [`CountryRestrictModule.sol`][trex-src].
//!
//! Recipients whose identity has a country code on the restriction list are
//! blocked from receiving tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryRestrictModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::modules::ComplianceModule;

/// Country Restriction Compliance Module Trait
///
/// The `CountryRestrict` trait extends the [`ComplianceModule`] trait to
/// provide a per-token country restriction list. When this module is
/// registered on a token's modular compliance contract, transfers and mints
/// are blocked whenever the recipient's identity (resolved via the Identity
/// Registry Storage) has any country code that appears in the token's
/// restriction list.
///
/// Only the recipient's country data is consulted. Although the underlying
/// compliance hooks ([`ComplianceModule::can_transfer`],
/// [`ComplianceModule::can_create`]) also receive the sender address and the
/// transfer amount, this module ignores both: a sender whose own countries
/// are on the restriction list can still send tokens out, and the size of
/// the transfer has no effect on the decision. If you need rules that
/// constrain the sender or scale with amount, those belong in a different
/// compliance module that you register alongside this one.
///
/// Countries are identified by ISO 3166-1 numeric codes and tracked
/// per-`token`, so a single compliance module contract can serve multiple
/// tokens with independent restriction lists.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait CountryRestrict: ComplianceModule {
    /// Configures the Identity Registry Storage contract for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose IRS is being configured.
    /// * `irs` - The Identity Registry Storage contract address.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`crate::rwa::compliance::modules::storage::set_irs_address`] for the
    /// implementation.
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address, operator: Address);

    /// Adds a country to the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `country` - The ISO 3166-1 numeric country code to restrict.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["country_restricted", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::add_country_restriction`] for the implementation.
    fn add_country_restriction(e: &Env, token: Address, country: u32, operator: Address);

    /// Removes a country from the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `country` - The ISO 3166-1 numeric country code to unrestrict.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["country_unrestricted", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::remove_country_restriction`] for the implementation.
    fn remove_country_restriction(e: &Env, token: Address, country: u32, operator: Address);

    /// Adds multiple countries to the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to restrict.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// For each country newly added to the restriction list:
    /// * topics - `["country_restricted", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::batch_restrict_countries`] for the implementation.
    ///
    /// Each `(token, country)` pair is stored in its own persistent entry, so
    /// the caller must size `countries` to stay within the per-transaction
    /// network limits — see <https://lab.stellar.org/network-limits>.
    fn batch_restrict_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address);

    /// Removes multiple countries from the restriction list for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to unrestrict.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// For each country removed from the restriction list:
    /// * topics - `["country_unrestricted", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::batch_unrestrict_countries`] for the implementation.
    ///
    /// Each `(token, country)` pair lives in its own persistent entry, so the
    /// caller must size `countries` to stay within the per-transaction network
    /// limits — see <https://lab.stellar.org/network-limits>.
    fn batch_unrestrict_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address);

    /// Returns `true` if `country` is restricted for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose restriction list is queried.
    /// * `country` - The ISO 3166-1 numeric country code to check.
    fn is_country_restricted(e: &Env, token: Address, country: u32) -> bool {
        storage::is_country_restricted(e, &token, country)
    }
}

// ################## EVENTS ##################

/// Emitted when a country is added to the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryRestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emits a [`CountryRestricted`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose restriction list changed.
/// * `country` - The ISO 3166-1 numeric country code that was restricted.
pub fn emit_country_restricted(e: &Env, token: &Address, country: u32) {
    CountryRestricted { token: token.clone(), country }.publish(e);
}

/// Emitted when a country is removed from the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnrestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emits a [`CountryUnrestricted`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose restriction list changed.
/// * `country` - The ISO 3166-1 numeric country code that was removed.
pub fn emit_country_unrestricted(e: &Env, token: &Address, country: u32) {
    CountryUnrestricted { token: token.clone(), country }.publish(e);
}
