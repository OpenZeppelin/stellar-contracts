use cvlr::nondet::*;
use cvlr_soroban::{nondet_address, nondet_bytes_n};
use soroban_sdk::{Address, BytesN, String, Vec};

// Helper trait to exclude BytesN<32> from the generic implementation
// This trait is implemented for all types that can use the generic Nondet implementation
pub(crate) trait CanUseGenericNondet {}

// Implement for common types that implement Nondet
impl CanUseGenericNondet for u32 {}
impl CanUseGenericNondet for bool {}
impl CanUseGenericNondet for String {}
impl CanUseGenericNondet for Vec<Address> {}
impl CanUseGenericNondet for Vec<u32> {}
// Add more types as needed
// Note: BytesN<32> is explicitly NOT implemented here, so it won't match the generic impl
// Note: Address is explicitly NOT implemented here, so it won't match the generic impl

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

impl <K: Clone + Eq, V> GhostMap<K, V> 
where 
    V: Nondet + Clone + CanUseGenericNondet
{
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

// Implementation for BytesN<32> using nondet_bytes_n()
// Note: We only implement for BytesN<32> here. If you need GhostMap for other BytesN sizes,
// add separate impl blocks for each size to avoid overlapping impl conflicts.
impl <K: Clone + Eq> GhostMap<K, BytesN<32>> {
    #[inline(never)]
    pub fn init(&mut self, k: &K, v: BytesN<32>) {
        *self = Self::Init { k: k.clone(), v: v.clone() };
    }

    #[inline(never)]
    pub fn set(&mut self, k: &K, v: BytesN<32>) {
        match self {
            Self::Init { k: my_k, v: my_v} =>
                if k == my_k {
                    *my_v = v
                }
            _ => {}
        }
    }

    #[inline(never)]
    pub fn get(&self, k: &K) -> BytesN<32> {
        match self {
            Self::UnInit => nondet_bytes_n(),
            Self::Init { k: my_k, v: my_v } => {
                if k == my_k {
                    my_v.clone()
                } else {
                    nondet_bytes_n()
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