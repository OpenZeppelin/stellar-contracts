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

use crate::rwa::compliance::ComplianceModule;

use super::common::{
    country_code, get_compliance_address, get_irs_client, module_name, require_compliance_auth,
    set_compliance_address, set_irs_address,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    AllowedCountry(Address, u32),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryAllowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnallowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

#[contract]
pub struct CountryAllowModule;

#[contractimpl]
impl CountryAllowModule {
    /// Configures the IRS address used for country lookups on `token`.
    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    pub fn add_allowed_country(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        e.storage()
            .persistent()
            .set(&DataKey::AllowedCountry(token.clone(), country), &true);
        CountryAllowed { token, country }.publish(e);
    }

    pub fn remove_allowed_country(e: &Env, token: Address, country: u32) {
        require_compliance_auth(e);
        e.storage()
            .persistent()
            .set(&DataKey::AllowedCountry(token.clone(), country), &false);
        CountryUnallowed { token, country }.publish(e);
    }

    pub fn batch_allow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            e.storage()
                .persistent()
                .set(&DataKey::AllowedCountry(token.clone(), country), &true);
            CountryAllowed { token: token.clone(), country }.publish(e);
        }
    }

    pub fn batch_disallow_countries(e: &Env, token: Address, countries: Vec<u32>) {
        require_compliance_auth(e);
        for country in countries.iter() {
            e.storage()
                .persistent()
                .set(&DataKey::AllowedCountry(token.clone(), country), &false);
            CountryUnallowed { token: token.clone(), country }.publish(e);
        }
    }

    pub fn is_country_allowed(e: &Env, token: Address, country: u32) -> bool {
        e.storage()
            .persistent()
            .get(&DataKey::AllowedCountry(token, country))
            .unwrap_or_default()
    }
}

#[contractimpl]
impl ComplianceModule for CountryAllowModule {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

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

#[cfg(test)]
mod test {
    use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env};

    use crate::rwa::{
        compliance::ComplianceModuleClient,
        identity_registry_storage::{
            CountryData, CountryRelation, IndividualCountryRelation,
        },
    };

    use super::CountryAllowModule;
    use crate::rwa::compliance::modules::test_utils::{MockIRS, MockIRSClient};

    #[contract]
    struct MockCompliance;

    #[test]
    fn country_allow_enforces_recipient_country() {
        let e = Env::default();
        e.mock_all_auths();

        let module = e.register(CountryAllowModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);

        let module_client = super::CountryAllowModuleClient::new(&e, &module);
        let irs_helper = MockIRSClient::new(&e, &irs);

        irs_helper.mock_set_countries(
            &to,
            &vec![
                &e,
                CountryData {
                    country: CountryRelation::Individual(
                        IndividualCountryRelation::Residence(840),
                    ),
                    metadata: None,
                },
            ],
        );

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.add_allowed_country(&token, &840);
        });

        assert!(client.can_transfer(&from, &to, &1, &token));

        e.as_contract(&compliance, || {
            module_client.remove_allowed_country(&token, &840);
        });
        assert!(!client.can_transfer(&from, &to, &1, &token));
    }

    #[test]
    fn allows_if_any_country_matches() {
        let e = Env::default();
        e.mock_all_auths();

        let module = e.register(CountryAllowModule, ());
        let token = Address::generate(&e);
        let compliance = e.register(MockCompliance, ());
        let irs = e.register(MockIRS, ());
        let from = Address::generate(&e);
        let to = Address::generate(&e);

        let client = ComplianceModuleClient::new(&e, &module);
        client.set_compliance_address(&compliance);

        let module_client = super::CountryAllowModuleClient::new(&e, &module);
        let irs_helper = MockIRSClient::new(&e, &irs);

        irs_helper.mock_set_countries(
            &to,
            &vec![
                &e,
                CountryData {
                    country: CountryRelation::Individual(
                        IndividualCountryRelation::Residence(840),
                    ),
                    metadata: None,
                },
                CountryData {
                    country: CountryRelation::Individual(
                        IndividualCountryRelation::Citizenship(276),
                    ),
                    metadata: None,
                },
            ],
        );

        e.as_contract(&compliance, || {
            module_client.set_identity_registry_storage(&token, &irs);
            module_client.add_allowed_country(&token, &276);
        });

        assert!(client.can_transfer(&from, &to, &1, &token));
    }
}
