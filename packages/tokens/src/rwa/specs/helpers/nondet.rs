use cvlr::nondet::{*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_symbol};
use soroban_sdk::{Address, Env, Map, String, Symbol, Vec};

use crate::rwa::{compliance::ComplianceHook, identity_registry_storage::{CountryData, CountryRelation, IndividualCountryRelation, OrganizationCountryRelation}};

pub fn nondet_vec_u32() -> Vec<u32> {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Vec<u32> = Vec::new(&env);
    let mut i = 0u32;
    while i < n {
        out.push_back(nondet());
        i += 1;
    }
    out
}

impl Nondet for ComplianceHook {
    fn nondet() -> Self {
        let choice: u32 = cvlr::nondet();
        match choice % 5 {
            0 => ComplianceHook::Transferred,
            1 => ComplianceHook::Created,
            2 => ComplianceHook::Destroyed,
            3 => ComplianceHook::CanTransfer,
            _ => ComplianceHook::CanCreate,
        }
    }
}

impl Nondet for IndividualCountryRelation {
    fn nondet() -> Self {
        let choice = u8::nondet() % 5;
        let country_code = u32::nondet() % 999; // ISO 3166-1 numeric code is a 3-digit number
        match choice {
            0 => IndividualCountryRelation::Residence(country_code),
            1 => IndividualCountryRelation::Citizenship(country_code),
            2 => IndividualCountryRelation::SourceOfFunds(country_code),
            3 => IndividualCountryRelation::TaxResidency(country_code),
            _ => {
                let symbol = nondet_symbol();
                IndividualCountryRelation::Custom(symbol, country_code)
            }
        }
    }
}

impl Nondet for CountryRelation {
    fn nondet() -> Self {
        let choice = u8::nondet() % 2;
        match choice {
            0 => CountryRelation::Individual(IndividualCountryRelation::nondet()),
            _ => CountryRelation::Organization(OrganizationCountryRelation::nondet()),
        }
    }
}


impl Nondet for OrganizationCountryRelation {
    fn nondet() -> Self {
        let choice = u8::nondet() % 5;
        let country_code = u32::nondet() % 999; // ISO 3166-1 numeric code is a 3-digit number
        match choice {
            0 => OrganizationCountryRelation::Incorporation(country_code),
            1 => OrganizationCountryRelation::OperatingJurisdiction(country_code),
            2 => OrganizationCountryRelation::TaxJurisdiction(country_code),
            3 => OrganizationCountryRelation::SourceOfFunds(country_code),
            _ => {
                let symbol = nondet_symbol();
                OrganizationCountryRelation::Custom(symbol, country_code)
            }
        }
    }
}

impl Nondet for CountryData {
    fn nondet() -> Self {
        let country_relation = CountryRelation::nondet();
        let metadata: Option<Map<Symbol, String>> = if bool::nondet() {
            Some(nondet_map())
        } else {
            None
        };
        Self { country: country_relation, metadata }
    }
}

pub fn nondet_vec_country() -> Vec<CountryData> {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Vec<CountryData> = Vec::new(&env);
    let mut i = 0u32;
    while i < n {
        let country_data = CountryData::nondet();
        out.push_back(nondet());
        i += 1;
    }
    out
}

pub fn nondet_vec_address() -> Vec<Address> {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Vec<Address> = Vec::new(&env);
    let mut i = 0u32;
    while i < n {
        out.push_back(nondet_address());
        i += 1;
    }
    out
}