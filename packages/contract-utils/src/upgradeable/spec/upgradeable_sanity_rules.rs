use cvlr::{cvlr_assert};
use cvlr_soroban_derive::rule;

use soroban_sdk::{Env};

use crate::upgradeable::*;

#[rule]
pub fn enable_migration_sanity(e: Env) {
    enable_migration(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn can_complete_migration_sanity(e: Env) {
    can_complete_migration(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn complete_migration_sanity(e: Env) {
    complete_migration(&e);
    cvlr_assert!(false);
}


#[rule]
pub fn ensure_can_complete_migration_sanity(e: Env) {
    ensure_can_complete_migration(&e);
    cvlr_assert!(false);
}