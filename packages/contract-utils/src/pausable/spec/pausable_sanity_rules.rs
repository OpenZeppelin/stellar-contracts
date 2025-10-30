use cvlr::{cvlr_assert};
use cvlr_soroban_derive::rule;

use soroban_sdk::{Env, Symbol};

use crate::pausable::*;

#[rule]
pub fn paused_sanity(e: Env) {
    paused(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn pause_sanity(e: Env) {
    pause(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn unpause_sanity(e: Env) {
    unpause(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn when_not_paused_sanity(e: Env) {
    when_not_paused(&e);
    cvlr_assert!(false);
}

#[rule]
pub fn when_paused_sanity(e: Env) {
    when_paused(&e);
    cvlr_assert!(false);
}