/// A leaderboard contract storing a `Score` per user in persistent storage.
/// This is the v1 version, whose `Score` struct will gain a `level` field in
/// v2. Because there can be many users, a batch migration would be impractical
/// — v2 uses lazy migration instead (entries are converted on first read).
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Symbol, Vec};
use stellar_access::access_control::{set_admin, AccessControl};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;

#[contracttype]
pub struct Score {
    pub points: u32,
}

#[contracttype]
pub enum StorageKey {
    Score(Address),
}

#[contract]
pub struct LeaderboardContract;

#[contractimpl]
impl LeaderboardContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }

    pub fn set_score(e: &Env, user: Address, points: u32) {
        e.storage().persistent().set(&StorageKey::Score(user), &Score { points });
    }

    pub fn get_score(e: &Env, user: Address) -> u32 {
        e.storage()
            .persistent()
            .get::<_, Score>(&StorageKey::Score(user))
            .map(|s| s.points)
            .unwrap_or(0)
    }
}

#[contractimpl]
impl Upgradeable for LeaderboardContract {
    #[only_role(operator, "manager")]
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for LeaderboardContract {}
