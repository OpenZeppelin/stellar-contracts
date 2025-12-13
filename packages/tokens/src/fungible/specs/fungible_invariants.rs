use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::fungible::{Base, FungibleToken};

// invariant total_supply >= balance
// we can't verify this without ghosts hooks.

// helpers
pub fn assume_pre_total_supply_geq_balance(e: Env, account: &Address) {
    clog!(cvlr_soroban::Addr(account));
    let total_supply = Base::total_supply(&e);
    clog!(total_supply);
    let balance = Base::balance(&e, account);
    clog!(balance);
    cvlr_assume!(total_supply >= balance);
}

pub fn assert_post_total_supply_geq_balance(e: Env, account: &Address) {
    clog!(cvlr_soroban::Addr(account));
    let total_supply = Base::total_supply(&e);
    clog!(total_supply);
    let balance = Base::balance(&e, account);
    clog!(balance);
    cvlr_assert!(total_supply >= balance);
}
