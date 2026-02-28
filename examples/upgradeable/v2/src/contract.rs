/// The contract in "v1" needs to be upgraded with this one. It demonstrates a
/// realistic storage migration: the `Config` struct gains a new `active` field,
/// so the `migrate()` function reads the old format, converts it, and writes
/// back in the new format. A schema version guard prevents double invocation.
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol, Vec,
};
use stellar_access::access_control::AccessControl;
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;

/// The old config type — field names and types must match what v1 stored.
#[contracttype]
pub struct ConfigV1 {
    pub rate: u32,
}

/// The new config type with an additional field.
#[contracttype]
pub struct Config {
    pub rate: u32,
    pub active: bool,
}

pub const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl Upgradeable for ExampleContract {
    #[only_role(operator, "manager")]
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[contractimpl]
impl ExampleContract {
    /// Migrates instance storage from v1 to v2 format. Reads the old `Config`
    /// (single `rate` field), converts it to the new shape (with `active`
    /// defaulting to `true`), and writes it back. A schema version prevents
    /// this from running twice.
    #[only_role(operator, "migrator")]
    pub fn migrate(e: &Env, operator: Address) {
        assert!(upgradeable::get_schema_version(e) < 2, "already migrated");

        let old: ConfigV1 = e.storage().instance().get(&CONFIG_KEY).unwrap();
        let new = Config { rate: old.rate, active: true };
        e.storage().instance().set(&CONFIG_KEY, &new);

        upgradeable::set_schema_version(e, 2);
    }

    pub fn get_rate(e: &Env) -> u32 {
        e.storage().instance().get::<_, Config>(&CONFIG_KEY).unwrap().rate
    }

    pub fn is_active(e: &Env) -> bool {
        e.storage().instance().get::<_, Config>(&CONFIG_KEY).unwrap().active
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ExampleContract {}
