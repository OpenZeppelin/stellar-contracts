/// The upgraded leaderboard contract. `Score` gains a `level` field, but
/// since there can be many users the migration is lazy: each entry is
/// converted from v1 format on its first read and immediately written back in
/// v2 format, so all subsequent reads skip the conversion entirely.
///
/// The `ScoreVersion` variant is new in v2 and absent for v1 entries;
/// reads that find no version marker default to version 1.
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Symbol, Vec};
use stellar_access::access_control::AccessControl;
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;

/// The old score type — field names and count must match what v1 stored.
#[contracttype]
pub struct ScoreV1 {
    pub points: u32,
}

/// The new score type with an additional `level` field.
#[contracttype]
pub struct Score {
    pub points: u32,
    pub level: u32,
}

#[contracttype]
pub enum StorageKey {
    Score(Address),
    // New in v2: records the schema version of each individual entry.
    // Absent for v1 entries, which default to version 1.
    ScoreVersion(Address),
}

#[contract]
pub struct LeaderboardContract;

#[contractimpl]
impl LeaderboardContract {
    pub fn set_score(e: &Env, user: Address, points: u32, level: u32) {
        set_score_inner(e, &user, &Score { points, level });
    }

    /// Returns the score for `user`. If the entry was written by v1 (no
    /// version marker), it is converted to v2 format and written back before
    /// being returned — this happens exactly once per entry.
    pub fn get_score(e: &Env, user: Address) -> Score {
        let version: u32 =
            e.storage().persistent().get(&StorageKey::ScoreVersion(user.clone())).unwrap_or(1);

        // MIGRATION HAPPENS LAZILY HERE
        match version {
            1 => {
                let v1: ScoreV1 =
                    e.storage().persistent().get(&StorageKey::Score(user.clone())).unwrap();
                let migrated = Score { points: v1.points, level: 0 };
                set_score_inner(e, &user, &migrated);
                migrated
            }
            _ => e.storage().persistent().get(&StorageKey::Score(user.clone())).unwrap(),
        }
    }
}

fn set_score_inner(e: &Env, user: &Address, score: &Score) {
    e.storage().persistent().set(&StorageKey::ScoreVersion(user.clone()), &2u32);
    e.storage().persistent().set(&StorageKey::Score(user.clone()), score);
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
