use soroban_sdk::{contracttype, BytesN, Symbol, Vec};

#[contracttype]
#[derive(Clone)]
pub struct FacetCut {
    // Should point to the uploaded (NOT deployed) facet's hash
    pub wasm_hash_of_facet: BytesN<32>,
    pub action: FacetAction,
    pub selectors: Vec<Symbol>,
    pub salt: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FacetAction {
    Add,
    Replace,
    Remove,
}
