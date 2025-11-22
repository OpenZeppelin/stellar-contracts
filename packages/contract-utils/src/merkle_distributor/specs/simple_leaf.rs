use cvlr::nondet::Nondet;
use soroban_sdk::contracttype;

use crate::merkle_distributor::IndexableLeaf;

// TODO: very basic. Expand as needed for other properties.

#[contracttype]
pub(crate) struct Leaf {
    pub index: u32
}

impl Nondet for Leaf {
    fn nondet() -> Self {
        Leaf {
            index: u32::nondet(),
        }
    }
}

impl IndexableLeaf for Leaf {
    fn index(&self) -> u32 {
        self.index
    }
}