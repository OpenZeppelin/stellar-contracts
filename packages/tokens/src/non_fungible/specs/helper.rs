use cvlr::clog;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{storage::NFTStorageKey, Base};

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

pub fn is_owned(e: &Env, token_id: u32) -> bool {
    let key = NFTStorageKey::Owner(token_id);
    let owner = e.storage().persistent().get::<_, Address>(&key);
    if let Some(owner_internal) = owner.clone() {
        clog!(cvlr_soroban::Addr(&owner_internal));
    }
    return owner.is_some();
}
