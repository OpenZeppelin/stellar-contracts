use soroban_sdk::{contracttype, Address, Env, Vec};

// TODO: refactor: do not store in instance and more efficient way of storing
// because num of tokens is unbound
#[contracttype]
#[derive(Clone)]
pub enum TokenBinderStorageKey {
    LinkedTokens,
}

pub fn get_linked_tokens(e: &Env) -> Vec<Address> {
    e.storage().instance().get(&TokenBinderStorageKey::LinkedTokens).unwrap_or(Vec::new(e))
}

fn set_linked_tokens(e: &Env, tokens: &Vec<Address>) {
    e.storage().instance().set(&TokenBinderStorageKey::LinkedTokens, tokens);
}

pub fn bind_token(e: &Env, token: &Address) {
    let mut tokens = get_linked_tokens(e);
    if tokens.iter().any(|t| t == *token) {
        // Token already bound, do nothing
        return;
    }
    tokens.push_back(token.clone());
    set_linked_tokens(e, &tokens);
}

pub fn unbind_token(e: &Env, token: &Address) {
    let mut tokens = get_linked_tokens(e);
    if let Some(pos) = tokens.iter().position(|t| t == *token) {
        tokens.remove(pos as u32);
    }
    set_linked_tokens(e, &tokens);
}
