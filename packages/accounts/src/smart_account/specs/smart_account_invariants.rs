use cvlr::clog;
use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, String, Val, Vec, map, panic_with_error, vec};

use crate::smart_account::{
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError, specs::{
        nondet::{nondet_policy_map, nondet_signers_vec},
        smart_account_contract::SmartAccountContract,
    }, storage::{self, SmartAccountStorageKey, get_persistent_entry, update_context_rule_valid_until}
};

// invariant: 
// any ctx rule has at least one signer or a policy.
// validate_signers_and_policies should always not panic.