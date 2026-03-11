//! Shared helpers for compliance modules.
//!
//! Centralizes compliance-address ownership/auth checks, safe arithmetic
//! guards, lightweight read-only client traits, and identity registry
//! storage (IRS) resolution helpers.

use soroban_sdk::{contractclient, contracttype, panic_with_error, Address, Env, String, Vec};

use crate::rwa::{
    compliance::{
        ComplianceClient, ComplianceHook, ComplianceModuleError, MODULE_EXTEND_AMOUNT,
        MODULE_TTL_THRESHOLD,
    },
    identity_registry_storage::{
        CountryData, CountryRelation, IndividualCountryRelation, OrganizationCountryRelation,
    },
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub enum ComplianceModuleStorageKey {
    /// Maps to the compliance contract address for this module instance.
    Compliance,
    /// Caches successful required-hook verification for this module instance.
    HooksVerified,
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
/// # Security Warning
///
/// This helper performs **no authorization checks**. It must only be called
/// during contract initialization or from entrypoints that are strictly
/// restricted to an admin or token owner. Exposing this as, or calling it
/// from, a publicly accessible module entrypoint would allow unauthorized
/// parties to bind the compliance contract for this module.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `compliance` - The address of the compliance contract.
///
/// # Errors
///
/// * [`ComplianceModuleError::ComplianceAlreadySet`] - When the compliance
///   address has already been set.
pub fn set_compliance_address(e: &Env, compliance: &Address) {
    let key = ComplianceModuleStorageKey::Compliance;
    if e.storage().instance().has(&key) {
        panic_with_error!(e, ComplianceModuleError::ComplianceAlreadySet);
    }
    e.storage().instance().set(&key, compliance);
}

/// Returns the stored compliance address.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`ComplianceModuleError::ComplianceNotSet`] - When no compliance contract
///   has been configured yet.
pub fn get_compliance_address(e: &Env) -> Address {
    let key = ComplianceModuleStorageKey::Compliance;
    if let Some(addr) = e.storage().instance().get::<_, Address>(&key) {
        addr
    } else {
        panic_with_error!(e, ComplianceModuleError::ComplianceNotSet)
    }
}

// ---------------------------------------------------------------------------
// Hook wiring verification
// ---------------------------------------------------------------------------

/// Returns `true` if the hook wiring has already been verified for this
/// module instance (cached after the first successful check).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn hooks_verified(e: &Env) -> bool {
    let key = ComplianceModuleStorageKey::HooksVerified;
    e.storage().instance().has(&key)
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
/// # Errors
///
/// * [`ComplianceModuleError::MissingRequiredHook`] - When any required hook is
///   not registered, which means the deployment is misconfigured and internal
///   state would drift.
pub fn verify_required_hooks(e: &Env, required: Vec<ComplianceHook>) {
    if hooks_verified(e) {
        return;
    }

    let ckey = ComplianceModuleStorageKey::Compliance;
    if !e.storage().instance().has(&ckey) {
        return;
    }

    let compliance: Address = e.storage().instance().get(&ckey).expect("compliance must be set");
    let self_addr = e.current_contract_address();
    let client = ComplianceClient::new(e, &compliance);

    for hook in required.iter() {
        if !client.is_module_registered(&hook, &self_addr) {
            panic_with_error!(e, ComplianceModuleError::MissingRequiredHook);
        }
    }

    let vkey = ComplianceModuleStorageKey::HooksVerified;
    e.storage().instance().set(&vkey, &true);
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

/// Adds two `i128` values, panicking on overflow.
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
pub fn add_i128_or_panic(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_add(right)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MathOverflow))
}

/// Subtracts two `i128` values, panicking on underflow.
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
pub fn sub_i128_or_panic(e: &Env, left: i128, right: i128) -> i128 {
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

/// Low-level helper that stores the IRS contract address for a given token.
///
/// This function **does not perform any authorization checks**. It directly
/// updates the per-token Identity Registry Storage pointer in persistent
/// storage.
///
/// SAFETY: This must only be called from initialization logic or from
/// admin-gated entrypoints that have already enforced the appropriate
/// ownership and authorization checks. Do **not** expose this helper directly
/// as a public contract method.
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
    use crate::rwa::{compliance::Compliance, utils::token_binder::TokenBinder};

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
    impl Compliance for MockComplianceContract {
        fn add_module_to(_e: &Env, _hook: ComplianceHook, _module: Address, _operator: Address) {
            unreachable!("add_module_to is not used in these tests");
        }

        fn remove_module_from(
            _e: &Env,
            _hook: ComplianceHook,
            _module: Address,
            _operator: Address,
        ) {
            unreachable!("remove_module_from is not used in these tests");
        }

        fn get_modules_for_hook(_e: &Env, _hook: ComplianceHook) -> Vec<Address> {
            unreachable!("get_modules_for_hook is not used in these tests");
        }

        fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
            let key = MockComplianceStorageKey::Registered(hook, module);
            e.storage().persistent().has(&key)
        }

        fn transferred(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {
            unreachable!("transferred is not used in these tests");
        }

        fn created(_e: &Env, _to: Address, _amount: i128, _token: Address) {
            unreachable!("created is not used in these tests");
        }

        fn destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {
            unreachable!("destroyed is not used in these tests");
        }

        fn can_transfer(
            _e: &Env,
            _from: Address,
            _to: Address,
            _amount: i128,
            _token: Address,
        ) -> bool {
            unreachable!("can_transfer is not used in these tests");
        }

        fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
            unreachable!("can_create is not used in these tests");
        }
    }

    #[contractimpl]
    impl TokenBinder for MockComplianceContract {
        fn linked_tokens(e: &Env) -> Vec<Address> {
            Vec::new(e)
        }

        fn bind_token(_e: &Env, _token: Address, _operator: Address) {
            unreachable!("bind_token is not used in these tests");
        }

        fn unbind_token(_e: &Env, _token: Address, _operator: Address) {
            unreachable!("unbind_token is not used in these tests");
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
    fn verify_required_hooks_returns_early_when_cached() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let compliance_id = e.register(MockComplianceContract, ());

        e.as_contract(&module_id, || {
            set_compliance_address(&e, &compliance_id);
            e.storage().instance().set(&ComplianceModuleStorageKey::HooksVerified, &true);

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
    #[should_panic(expected = "Error(Contract, #399)")]
    fn set_compliance_address_panics_with_contract_error_when_already_set() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let compliance_id = e.register(MockComplianceContract, ());

        e.as_contract(&module_id, || {
            set_compliance_address(&e, &compliance_id);
            set_compliance_address(&e, &compliance_id);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #390)")]
    fn get_compliance_address_panics_when_not_configured() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());

        e.as_contract(&module_id, || {
            let _ = get_compliance_address(&e);
        });
    }

    #[test]
    fn get_compliance_address_returns_configured_address() {
        let e = Env::default();
        let module_id = e.register(MockModuleContract, ());
        let compliance_id = e.register(MockComplianceContract, ());

        e.as_contract(&module_id, || {
            set_compliance_address(&e, &compliance_id);

            assert_eq!(get_compliance_address(&e), compliance_id);
        });
    }

    #[test]
    fn panicking_math_helpers_return_expected_values() {
        let e = Env::default();

        assert_eq!(add_i128_or_panic(&e, 2, 3), 5);
        assert_eq!(sub_i128_or_panic(&e, 7, 4), 3);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #392)")]
    fn add_i128_or_panic_panics_on_overflow() {
        let e = Env::default();

        let _ = add_i128_or_panic(&e, i128::MAX, 1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #393)")]
    fn sub_i128_or_panic_panics_on_underflow() {
        let e = Env::default();

        let _ = sub_i128_or_panic(&e, i128::MIN, 1);
    }
}
