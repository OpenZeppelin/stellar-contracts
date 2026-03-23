//! Country allowlist compliance module — Stellar port of T-REX
//! [`CountryAllowModule.sol`][trex-src].
//!
//! Only recipients whose identity has at least one country code in the
//! allowlist may receive tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryAllowModule.sol

pub mod storage;

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

#[cfg(test)]
mod test {
    extern crate std;

    use soroban_sdk::{
        contract, contractimpl, contracttype, testutils::Address as _, vec, Address, Env, IntoVal,
        Val, Vec,
    };

    use super::*;
    use crate::rwa::{
        identity_registry_storage::{
            CountryData, CountryDataManager, CountryRelation, IdentityRegistryStorage,
            IndividualCountryRelation, OrganizationCountryRelation,
        },
        utils::token_binder::TokenBinder,
    };

    #[contract]
    struct MockIRSContract;

    #[contracttype]
    #[derive(Clone)]
    enum MockIRSStorageKey {
        Identity(Address),
        CountryEntries(Address),
    }

    #[contractimpl]
    impl TokenBinder for MockIRSContract {
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
    impl IdentityRegistryStorage for MockIRSContract {
        fn add_identity(
            _e: &Env,
            _account: Address,
            _identity: Address,
            _country_data_list: Vec<Val>,
            _operator: Address,
        ) {
            unreachable!("add_identity is not used in these tests");
        }

        fn remove_identity(_e: &Env, _account: Address, _operator: Address) {
            unreachable!("remove_identity is not used in these tests");
        }

        fn modify_identity(_e: &Env, _account: Address, _identity: Address, _operator: Address) {
            unreachable!("modify_identity is not used in these tests");
        }

        fn recover_identity(
            _e: &Env,
            _old_account: Address,
            _new_account: Address,
            _operator: Address,
        ) {
            unreachable!("recover_identity is not used in these tests");
        }

        fn stored_identity(e: &Env, account: Address) -> Address {
            e.storage()
                .persistent()
                .get(&MockIRSStorageKey::Identity(account.clone()))
                .unwrap_or(account)
        }
    }

    #[contractimpl]
    impl CountryDataManager for MockIRSContract {
        fn add_country_data_entries(
            _e: &Env,
            _account: Address,
            _country_data_list: Vec<Val>,
            _operator: Address,
        ) {
            unreachable!("add_country_data_entries is not used in these tests");
        }

        fn modify_country_data(
            _e: &Env,
            _account: Address,
            _index: u32,
            _country_data: Val,
            _operator: Address,
        ) {
            unreachable!("modify_country_data is not used in these tests");
        }

        fn delete_country_data(_e: &Env, _account: Address, _index: u32, _operator: Address) {
            unreachable!("delete_country_data is not used in these tests");
        }

        fn get_country_data_entries(e: &Env, account: Address) -> Vec<Val> {
            let entries: Vec<CountryData> = e
                .storage()
                .persistent()
                .get(&MockIRSStorageKey::CountryEntries(account))
                .unwrap_or_else(|| Vec::new(e));

            Vec::from_iter(e, entries.iter().map(|entry| entry.into_val(e)))
        }
    }

    #[contractimpl]
    impl MockIRSContract {
        pub fn set_country_data_entries(e: &Env, account: Address, entries: Vec<CountryData>) {
            e.storage().persistent().set(&MockIRSStorageKey::CountryEntries(account), &entries);
        }
    }

    #[contract]
    struct TestCountryAllowContract;

    #[contractimpl(contracttrait)]
    impl CountryAllow for TestCountryAllowContract {
        fn set_compliance_address(_e: &Env, _compliance: Address) {
            unreachable!("set_compliance_address is not used in these tests");
        }
    }

    fn individual_country(code: u32) -> CountryData {
        CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(code)),
            metadata: None,
        }
    }

    fn organization_country(code: u32) -> CountryData {
        CountryData {
            country: CountryRelation::Organization(
                OrganizationCountryRelation::OperatingJurisdiction(code),
            ),
            metadata: None,
        }
    }

    #[test]
    fn can_transfer_and_create_allow_when_any_country_matches() {
        let e = Env::default();
        let module_id = e.register(TestCountryAllowContract, ());
        let irs_id = e.register(MockIRSContract, ());
        let irs = MockIRSContractClient::new(&e, &irs_id);
        let token = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        irs.set_country_data_entries(
            &to,
            &vec![&e, individual_country(250), organization_country(276)],
        );

        e.as_contract(&module_id, || {
            set_irs_address(&e, &token, &irs_id);
            set_country_allowed(&e, &token, 276);

            assert!(<TestCountryAllowContract as CountryAllow>::can_transfer(
                &e,
                from.clone(),
                to.clone(),
                100,
                token.clone(),
            ));
            assert!(<TestCountryAllowContract as CountryAllow>::can_create(
                &e,
                to.clone(),
                100,
                token.clone(),
            ));
        });
    }

    #[test]
    fn can_transfer_and_create_reject_when_no_country_matches() {
        let e = Env::default();
        let module_id = e.register(TestCountryAllowContract, ());
        let irs_id = e.register(MockIRSContract, ());
        let irs = MockIRSContractClient::new(&e, &irs_id);
        let token = Address::generate(&e);
        let from = Address::generate(&e);
        let empty_to = Address::generate(&e);
        let disallowed_to = Address::generate(&e);

        irs.set_country_data_entries(&disallowed_to, &vec![&e, individual_country(250)]);

        e.as_contract(&module_id, || {
            set_irs_address(&e, &token, &irs_id);
            set_country_allowed(&e, &token, 276);

            assert!(!<TestCountryAllowContract as CountryAllow>::can_transfer(
                &e,
                from.clone(),
                empty_to.clone(),
                100,
                token.clone(),
            ));
            assert!(!<TestCountryAllowContract as CountryAllow>::can_create(
                &e,
                empty_to,
                100,
                token.clone(),
            ));

            assert!(!<TestCountryAllowContract as CountryAllow>::can_transfer(
                &e,
                from.clone(),
                disallowed_to.clone(),
                100,
                token.clone(),
            ));
            assert!(!<TestCountryAllowContract as CountryAllow>::can_create(
                &e,
                disallowed_to,
                100,
                token.clone(),
            ));
        });
    }
}
