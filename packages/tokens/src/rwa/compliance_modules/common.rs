//! Shared helpers for compliance modules.
//!
//! Centralizes compliance-address ownership/auth checks, safe arithmetic
//! guards, lightweight read-only client traits, and identity registry
//! storage (IRS) resolution helpers.

use soroban_sdk::{
    contractclient, contracttype, panic_with_error, Address, Env, String, Symbol, Vec,
};

use crate::rwa::{
    compliance::ComplianceHook,
    compliance_modules::{ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD},
    identity_registry_storage::{
        CountryData, CountryRelation, IndividualCountryRelation, OrganizationCountryRelation,
    },
};

// ---------------------------------------------------------------------------
// Storage key helpers — use Symbol::new() with full names per repo convention
// ---------------------------------------------------------------------------

fn compliance_key(e: &Env) -> Symbol {
    Symbol::new(e, "compliance_address")
}

fn hooks_verified_key(e: &Env) -> Symbol {
    Symbol::new(e, "hooks_verified")
}

/// Read-only cross-contract client into the Identity Registry Storage.
///
/// Modules that need identity or country resolution store the IRS address
/// per token and call through this client at check time — mirroring the
/// T-REX pattern where modules resolve identity via the token's registry.
#[contractclient(name = "IRSReadClient")]
pub trait IRSRead {
    /// Returns the on-chain identity address associated with `account`.
    fn stored_identity(e: &Env, account: Address) -> Address;

    /// Returns all country data entries for `account`.
    fn get_country_data_entries(e: &Env, account: Address) -> Vec<CountryData>;
}

/// Storage key for identity registry storage address, scoped per token.
#[contracttype]
#[derive(Clone)]
pub enum IRSKey {
    /// The IRS contract address for a specific token.
    Registry(Address),
}

// ---------------------------------------------------------------------------
// Compliance address management
// ---------------------------------------------------------------------------

/// Persists the compliance contract address that governs this module.
///
/// This is a **one-time** operation. Once set, the compliance address cannot
/// be changed. This prevents unauthorized rebinding after initial deployment.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `compliance` - The address of the compliance contract.
///
/// # Panics
///
/// Panics if the compliance address has already been set.
pub fn set_compliance_address(e: &Env, compliance: &Address) {
    let key = compliance_key(e);
    if e.storage().persistent().has(&key) {
        panic!("compliance address already set");
    }
    e.storage().persistent().set(&key, compliance);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the stored compliance address.
///
/// Falls back to the module's own address when no compliance contract has
/// been configured yet — this allows admin configuration before locking.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Returns
///
/// The compliance contract [`Address`], or the module's own address if not
/// yet configured.
pub fn get_compliance_address(e: &Env) -> Address {
    let key = compliance_key(e);
    if let Some(addr) = e.storage().persistent().get::<_, Address>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        addr
    } else {
        e.current_contract_address()
    }
}

/// Requires authorization from the compliance contract and returns its
/// address.
///
/// Before `set_compliance_address` is called, falls back to self-auth
/// (returns the module's own address without requiring external auth).
/// This allows admin operations during initial configuration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Returns
///
/// The compliance contract [`Address`].
pub fn require_compliance_auth(e: &Env) -> Address {
    let key = compliance_key(e);
    if let Some(compliance) = e.storage().persistent().get::<_, Address>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        compliance.require_auth();
        compliance
    } else {
        e.current_contract_address()
    }
}

// ---------------------------------------------------------------------------
// Hook wiring verification
// ---------------------------------------------------------------------------

/// Minimal read-only client for querying the compliance contract's
/// hook registrations. Only exposes the `is_module_registered` view.
#[contractclient(name = "ComplianceHookCheckClient")]
pub trait ComplianceHookCheck {
    /// Checks if `module` is registered for `hook` on the compliance contract.
    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool;
}

/// Returns `true` if the hook wiring has already been verified for this
/// module instance (cached after the first successful check).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn hooks_verified(e: &Env) -> bool {
    let key = hooks_verified_key(e);
    let verified = e.storage().persistent().has(&key);
    if verified {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
    }
    verified
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on every hook in `required`. Caches the result on success
/// so subsequent calls are a single storage read.
///
/// Skips verification if `set_compliance_address` has not been called
/// yet (the module is in unconfigured mode).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `required` - The list of hooks this module requires to be registered.
///
/// # Panics
///
/// Panics with [`ComplianceModuleError::MissingRequiredHook`] if any
/// required hook is not registered — this means the deployment is
/// misconfigured and internal state would drift.
pub fn verify_required_hooks(e: &Env, required: Vec<ComplianceHook>) {
    let ckey = compliance_key(e);
    if !e.storage().persistent().has(&ckey) {
        return;
    }

    let compliance: Address = e.storage().persistent().get(&ckey).expect("compliance must be set");
    let self_addr = e.current_contract_address();
    let client = ComplianceHookCheckClient::new(e, &compliance);

    for i in 0..required.len() {
        let hook = required.get(i).unwrap();
        if !client.is_module_registered(&hook, &self_addr) {
            panic_with_error!(e, ComplianceModuleError::MissingRequiredHook);
        }
    }

    let vkey = hooks_verified_key(e);
    e.storage().persistent().set(&vkey, &true);
    e.storage().persistent().extend_ttl(&vkey, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

// ---------------------------------------------------------------------------
// Amount validation
// ---------------------------------------------------------------------------

/// Panics with [`ComplianceModuleError::InvalidAmount`] if `amount` is
/// negative.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The amount to validate.
pub fn require_non_negative_amount(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, ComplianceModuleError::InvalidAmount);
    }
}

/// Checked `i128` addition.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `left` - The left operand.
/// * `right` - The right operand.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathOverflow`] - When the addition overflows.
pub fn checked_add_i128(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_add(right)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MathOverflow))
}

/// Checked `i128` subtraction.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `left` - The left operand.
/// * `right` - The right operand.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathUnderflow`] - When the subtraction
///   underflows.
pub fn checked_sub_i128(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_sub(right)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MathUnderflow))
}

/// Allocates a Soroban [`String`] from a static `&str` for use as a
/// module name.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The name to convert.
pub fn module_name(e: &Env, name: &str) -> String {
    String::from_str(e, name)
}

// ---------------------------------------------------------------------------
// Identity Registry Storage helpers
// ---------------------------------------------------------------------------

/// Stores the IRS contract address for a given token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS is being configured.
/// * `irs` - The IRS contract address.
pub fn set_irs_address(e: &Env, token: &Address, irs: &Address) {
    let key = IRSKey::Registry(token.clone());
    e.storage().persistent().set(&key, irs);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns an IRS cross-contract client for the given token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS client is requested.
///
/// # Errors
///
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for this token.
pub fn get_irs_client<'a>(e: &'a Env, token: &Address) -> IRSReadClient<'a> {
    let key = IRSKey::Registry(token.clone());
    let irs: Address = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::IdentityRegistryNotSet));
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
    IRSReadClient::new(e, &irs)
}

/// Extracts the numeric ISO 3166-1 country code from any
/// [`CountryRelation`] variant, regardless of individual/organization type.
///
/// # Arguments
///
/// * `relation` - The country relation to extract the code from.
pub fn country_code(relation: &CountryRelation) -> u32 {
    match relation {
        CountryRelation::Individual(rel) => match rel {
            IndividualCountryRelation::Residence(c)
            | IndividualCountryRelation::Citizenship(c)
            | IndividualCountryRelation::SourceOfFunds(c)
            | IndividualCountryRelation::TaxResidency(c) => *c,
            IndividualCountryRelation::Custom(_, c) => *c,
        },
        CountryRelation::Organization(rel) => match rel {
            OrganizationCountryRelation::Incorporation(c)
            | OrganizationCountryRelation::OperatingJurisdiction(c)
            | OrganizationCountryRelation::TaxJurisdiction(c)
            | OrganizationCountryRelation::SourceOfFunds(c) => *c,
            OrganizationCountryRelation::Custom(_, c) => *c,
        },
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use soroban_sdk::{contract, contractimpl, testutils::Address as _, vec, Address, Env, Vec};

    use super::*;

    #[contract]
    struct MockModuleContract;

    #[contract]
    struct MockComplianceContract;

    #[contracttype]
    #[derive(Clone)]
    enum MockComplianceStorageKey {
        Registered(ComplianceHook, Address),
    }

    #[contractimpl]
    impl ComplianceHookCheck for MockComplianceContract {
        fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
            let key = MockComplianceStorageKey::Registered(hook, module);
            e.storage().persistent().has(&key)
        }
    }

    #[contractimpl]
    impl MockComplianceContract {
        pub fn register_hook(e: &Env, hook: ComplianceHook, module: Address) {
            let key = MockComplianceStorageKey::Registered(hook, module);
            e.storage().persistent().set(&key, &true);
        }
    }

    #[contract]
    struct MockIRSContract;

    #[contractimpl]
    impl IRSRead for MockIRSContract {
        fn stored_identity(_e: &Env, account: Address) -> Address {
            account
        }

        fn get_country_data_entries(e: &Env, _account: Address) -> Vec<CountryData> {
            Vec::new(e)
        }
    }

    #[test]
    fn verify_required_hooks_skips_when_unconfigured() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());

        e.as_contract(&module_id, || {
            verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);

            assert!(!hooks_verified(&e));
        });
    }

    #[test]
    fn verify_required_hooks_sets_cache_when_registered() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let compliance_id = e.register(MockComplianceContract, ());
        let compliance = MockComplianceContractClient::new(&e, &compliance_id);

        compliance.register_hook(&ComplianceHook::CanTransfer, &module_id);

        e.as_contract(&module_id, || {
            set_compliance_address(&e, &compliance_id);

            verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);

            assert!(hooks_verified(&e));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #398)")]
    fn verify_required_hooks_missing_required_hook_panics_with_contract_error() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let compliance_id = e.register(MockComplianceContract, ());

        e.as_contract(&module_id, || {
            set_compliance_address(&e, &compliance_id);

            verify_required_hooks(&e, vec![&e, ComplianceHook::CanTransfer]);
        });
    }

    #[test]
    fn get_irs_client_returns_working_client_for_configured_token() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let irs_id = e.register(MockIRSContract, ());
        let token = Address::generate(&e);
        let account = Address::generate(&e);

        e.as_contract(&module_id, || {
            set_irs_address(&e, &token, &irs_id);

            let client = get_irs_client(&e, &token);
            assert_eq!(client.stored_identity(&account), account);
            assert_eq!(client.get_country_data_entries(&account).len(), 0);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #397)")]
    fn get_irs_client_panics_when_not_configured() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let token = Address::generate(&e);

        e.as_contract(&module_id, || {
            let _ = get_irs_client(&e, &token);
        });
    }

    #[test]
    fn checked_math_helpers_return_expected_values() {
        let e = Env::default();

        assert_eq!(checked_add_i128(&e, 2, 3), 5);
        assert_eq!(checked_sub_i128(&e, 7, 4), 3);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #392)")]
    fn checked_add_i128_panics_on_overflow() {
        let e = Env::default();

        let _ = checked_add_i128(&e, i128::MAX, 1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #393)")]
    fn checked_sub_i128_panics_on_underflow() {
        let e = Env::default();

        let _ = checked_sub_i128(&e, i128::MIN, 1);
    }
}
