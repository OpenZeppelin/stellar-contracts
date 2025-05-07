pub mod storage;

mod test;

use soroban_sdk::{Address, Env};

pub trait SACAdmin {
    fn mint(e: Env, to: Address, amount: i128, operator: Address);

    fn set_admin(e: Env, new_admin: Address, operator: Address);

    fn set_authorized(e: Env, id: Address, authorize: bool, operator: Address);

    fn clawback(e: Env, from: Address, amount: i128, operator: Address);
}
