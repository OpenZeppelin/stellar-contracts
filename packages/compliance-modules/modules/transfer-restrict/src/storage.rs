use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum TransferRestrictStorageKey {
    /// Per-(token, address) allowlist flag.
    AllowedUser(Address, Address),
}

pub fn is_user_allowed(e: &Env, token: &Address, user: &Address) -> bool {
    e.storage()
        .persistent()
        .get(&TransferRestrictStorageKey::AllowedUser(token.clone(), user.clone()))
        .unwrap_or_default()
}

pub fn set_user_allowed(e: &Env, token: &Address, user: &Address) {
    e.storage()
        .persistent()
        .set(&TransferRestrictStorageKey::AllowedUser(token.clone(), user.clone()), &true);
}

pub fn remove_user_allowed(e: &Env, token: &Address, user: &Address) {
    e.storage()
        .persistent()
        .remove(&TransferRestrictStorageKey::AllowedUser(token.clone(), user.clone()));
}
