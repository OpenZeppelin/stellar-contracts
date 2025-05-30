#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::ContractContext, contract, testutils::Address as _, Address, Env, IntoVal, Symbol,
};
use soroban_test_helpers;

use super::{extract_sac_contract_context, get_sac_address, set_sac_address, SacFn};
use crate::sac_admin_generic::storage::SACAdminGenericDataKey;

#[contract]
struct MockContract;

#[soroban_test_helpers::test]
fn test_set_and_get_sac_address(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let sac_addr: Address = e.storage().instance().get(&SACAdminGenericDataKey::Sac).unwrap();
        assert_eq!(get_sac_address(&e), sac);
        assert_eq!(sac_addr, sac);
    });
}

#[soroban_test_helpers::test]
#[should_panic(expected = "Error(Contract, #109)")]
fn test_sac_get_address_fails(e: Env) {
    let new_admin = e.register(MockContract, ());

    e.as_contract(&new_admin, || get_sac_address(&e));
}

#[soroban_test_helpers::test]
fn test_extract_context_mint(e: Env, sac: Address, to: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "mint"),
        args: ((), to, 1000i128).into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let res = extract_sac_contract_context(&e, &context);
        assert!(matches!(res, SacFn::Mint(v) if 1000i128 == v))
    });
}

#[soroban_test_helpers::test]
fn test_extract_context_clawback(e: Env, sac: Address, from: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "clawback"),
        args: ((), from, 2000i128).into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let res = extract_sac_contract_context(&e, &context);
        assert!(matches!(res, SacFn::Clawback(v) if v == 2000i128));
    });
}

#[soroban_test_helpers::test]
fn test_extract_context_set_authorized(e: Env, sac: Address, user: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "set_authorized"),
        args: ((), true, user).into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let res = extract_sac_contract_context(&e, &context);
        assert!(matches!(res, SacFn::SetAuthorized(v) if v));
    });
}

#[soroban_test_helpers::test]
fn test_extract_context_set_admin(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "set_admin"),
        args: ().into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let res = extract_sac_contract_context(&e, &context);
        assert!(matches!(res, SacFn::SetAdmin));
    });
}

#[soroban_test_helpers::test]
fn test_extract_context_unknown_fn(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "nonexistent"),
        args: ().into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let res = extract_sac_contract_context(&e, &context);
        assert!(matches!(res, SacFn::Unknown));
    });
}

#[soroban_test_helpers::test]
#[should_panic(expected = "Error(Contract, #110)")] // SACMissingFnParam
fn test_extract_context_address_mismatch(e: Env, sac: Address, other: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: other,
        fn_name: Symbol::new(&e, "mint"),
        args: ().into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let _ = extract_sac_contract_context(&e, &context);
    });
}

#[soroban_test_helpers::test]
#[should_panic(expected = "Error(Contract, #111)")] // SACMissingFnParam
fn test_extract_context_missing_param(e: Env, sac: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "mint"),
        args: ().into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let _ = extract_sac_contract_context(&e, &context);
    });
}

#[soroban_test_helpers::test]
#[should_panic(expected = "Error(Contract, #112)")] // SACInvalidFnParam
fn test_extract_context_invalid_param_type(e: Env, sac: Address, to: Address) {
    let new_admin = e.register(MockContract, ());

    let context = ContractContext {
        contract: sac.clone(),
        fn_name: Symbol::new(&e, "mint"),
        args: ((), to, Symbol::new(&e, "not_a_number")).into_val(&e),
    };

    e.as_contract(&new_admin, || {
        set_sac_address(&e, &sac);
        let _ = extract_sac_contract_context(&e, &context);
    });
}
