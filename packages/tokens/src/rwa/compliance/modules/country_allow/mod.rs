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

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::modules::ComplianceModule;

/// Country Allowlist Compliance Module Trait
///
/// The `CountryAllow` trait extends the [`ComplianceModule`] trait to provide
/// a per-token country allowlist. When this module is registered on a token's
/// modular compliance contract, transfers and mints are permitted only when
/// the recipient's identity (resolved via the Identity Registry Storage) has
/// at least one country code that appears in the token's allowlist.
///
/// Only the recipient's country data is consulted. Although the underlying
/// compliance hooks ([`ComplianceModule::on_transfer`],
/// [`ComplianceModule::on_created`]) also receive the sender address and the
/// transfer amount, this module ignores both: a sender whose own countries
/// are not on the allowlist can still send tokens out, and the size of the
/// transfer has no effect on the decision. If you need rules that constrain
/// the sender or scale with amount, those belong in a different compliance
/// module that you register alongside this one.
///
/// Countries are identified by ISO 3166-1 numeric codes and tracked
/// per-`token`, so a single compliance module contract can serve multiple
/// tokens with independent allowlists.
///
/// # Matching semantics and limitations
///
/// A recipient is admitted as soon as **one** country code on its identity
/// appears in the allowlist; the remaining entries are not examined. The
/// match is on the bare numeric country code alone, which has three
/// consequences that stricter deployments must address with an additional
/// compliance module registered alongside this one:
///
/// - **Any-match, not all-match.** A holder that is also tied to a
///   non-allowlisted jurisdiction still passes, since a single allowed tie is
///   sufficient. This module cannot express "every country on the identity must
///   be allowed".
/// - **Relation type is not distinguished.** The lookup uses only the numeric
///   code, so every [`crate::rwa::identity_registry_storage::CountryRelation`]
///   variant is treated identically — both the individual relations (residence,
///   citizenship, source of funds, tax residency, custom) and the organization
///   relations (incorporation, operating jurisdiction, tax jurisdiction, source
///   of funds, custom). A country attached only as, say, source of funds is
///   therefore accepted exactly as if it were the country of residence or
///   incorporation.
/// - **Entry validity metadata is not honored.** The per-entry
///   [`crate::rwa::identity_registry_storage::CountryData::metadata`],
///   documented as carrying a validity period such as a visa, is never
///   consulted, so an expired tie stays eligible until an issuer removes the
///   country data entry from the Identity Registry Storage.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait CountryAllow: ComplianceModule {
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

    /// Adds a country to the allowlist for `token`. If `country` is already
    /// allowed, the call is a no-op (no event emitted, no error raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `country` - The ISO 3166-1 numeric country code to allow.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["country_allowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::add_allowed_country`]
    /// for the implementation.
    fn add_allowed_country(e: &Env, token: Address, country: u32, operator: Address);

    /// Removes a country from the allowlist for `token`. If `country` is not
    /// currently allowed, the call is a no-op (no event emitted, no error
    /// raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `country` - The ISO 3166-1 numeric country code to remove.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["country_unallowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::remove_allowed_country`] for the implementation.
    fn remove_allowed_country(e: &Env, token: Address, country: u32, operator: Address);

    /// Adds multiple countries to the allowlist for `token`. Entries that are
    /// already allowed are silently skipped (no event emitted, no error
    /// raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to allow.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// For each country newly added to the allowlist:
    /// * topics - `["country_allowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::batch_allow_countries`]
    /// for the implementation.
    ///
    /// Each `(token, country)` pair is stored in its own persistent entry, so
    /// the caller must size `countries` to stay within the per-transaction
    /// network limits — see <https://lab.stellar.org/network-limits>.
    fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address);

    /// Removes multiple countries from the allowlist for `token`. Entries
    /// that are not currently allowed are silently skipped (no event emitted,
    /// no error raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `countries` - The ISO 3166-1 numeric country codes to remove.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// For each country removed from the allowlist:
    /// * topics - `["country_unallowed", token: Address]`
    /// * data - `[country: u32]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::batch_disallow_countries`] for the implementation.
    ///
    /// Each `(token, country)` pair lives in its own persistent entry, so the
    /// caller must size `countries` to stay within the per-transaction network
    /// limits — see <https://lab.stellar.org/network-limits>.
    fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>, operator: Address);

    /// Returns `true` if `country` is allowed for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is queried.
    /// * `country` - The ISO 3166-1 numeric country code to check.
    fn is_country_allowed(e: &Env, token: Address, country: u32) -> bool {
        storage::is_country_allowed(e, &token, country)
    }
}

// ################## EVENTS ##################

/// Emitted when a country is added to the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryAllowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emits a [`CountryAllowed`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose allowlist changed.
/// * `country` - The ISO 3166-1 numeric country code that was allowed.
pub fn emit_country_allowed(e: &Env, token: &Address, country: u32) {
    CountryAllowed { token: token.clone(), country }.publish(e);
}

/// Emitted when a country is removed from the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnallowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emits a [`CountryUnallowed`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose allowlist changed.
/// * `country` - The ISO 3166-1 numeric country code that was removed.
pub fn emit_country_unallowed(e: &Env, token: &Address, country: u32) {
    CountryUnallowed { token: token.clone(), country }.publish(e);
}
