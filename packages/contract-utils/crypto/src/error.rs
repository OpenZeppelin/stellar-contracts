use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CryptoError {
    /// The merkle proof length is out of bounds.
    MerkleProofOutOfBounds = 1400,
    /// The index of the leaf is out of bounds.
    MerkleIndexOutOfBounds = 1401,
    /// No data in hasher state.
    HasherEmptyState = 1402,
}
