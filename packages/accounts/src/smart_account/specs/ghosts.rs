use cvlr::nondet::*;

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