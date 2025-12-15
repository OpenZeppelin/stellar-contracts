use cvlr::nondet::*;
use cvlr_soroban::nondet_address;
use soroban_sdk::Address;

// ghost variable that holds a single value.
pub enum GhostVar<V> {
    UnInit,
    Init { v: V }
}

// Implementation for Address using nondet_address()
// Note: We only implement for Address here. If you need GhostVar for other types,
// add separate impl blocks for each type to avoid overlapping impl conflicts.
impl GhostVar<Address> {
    #[inline(never)]
    pub fn init(&mut self, v: Address) {
        *self = Self::Init { v };
    }

    #[inline(never)]
    pub fn set(&mut self, v: Address) {
        match self {
            Self::Init { v: my_v } => *my_v = v,
            _ => {}
        }
    }

    #[inline(never)]
    pub fn get(&self) -> Address {
        match self {
            Self::UnInit => nondet_address(),
            Self::Init { v: my_v } => my_v.clone()
        }
    }
}

// ghost map that can hold just one entry.
pub enum GhostMap<K, V> {
    UnInit,
    Init { k: K, v: V }
}

impl <K: Clone + Eq, V: Nondet + Clone> GhostMap<K, V> {
    #[inline(never)]
    pub fn init(&mut self, k: &K, v: V) {
        *self = Self::Init { k: k.clone(), v: v.clone() };
    }

    #[inline(never)]
    pub fn set(&mut self, k: &K, v: V) {
        match self {
            Self::Init { k: my_k, v: my_v} =>
                if k == my_k {
                    *my_v = v
                }
            _ => {}
        }
    }

    #[inline(never)]
    pub fn get(&self, k: &K) -> V {
        match self {
            Self::UnInit => V::nondet(),
            Self::Init { k: my_k, v: my_v } => {
                if k == my_k {
                    my_v.clone()
                } else {
                    V::nondet()
                }
            }
        }
    }
}

// policy that will have 1 storage variable
// install - sets storage var.
// uninstall
// can_enforce - ghost depending on storage variable and inputs
// enforce - using can_enforce
// set_storage_varaible