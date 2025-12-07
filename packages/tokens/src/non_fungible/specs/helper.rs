use soroban_sdk::{Env, Address};
use crate::non_fungible::Base;  
use cvlr::clog;

pub fn is_approved_for_token(e: &Env, owner: &Address, operator: &Address, token_id: u32) -> bool {
    let get_approved_result = Base::get_approved(e, token_id);
    if let Some(ref approved) = get_approved_result {
        clog!(cvlr_soroban::Addr(approved));
    }
    let is_approved_for_all_result = Base::is_approved_for_all(e, owner, operator);
    clog!(is_approved_for_all_result);
    if owner == operator {
        return true;
    }
    if get_approved_result.as_ref() == Some(operator) {
        return true;
    }
    if is_approved_for_all_result {
        return true;
    }
    false
}