use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{self as compliance_storage},
        supply_limit::{storage as supply_limit, SupplyLimit},
        ComplianceModule,
    },
    AccountSnapshot,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct SupplyLimitContract;

#[contractimpl]
impl SupplyLimitContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for SupplyLimitContract {}

#[contractimpl(contracttrait)]
impl SupplyLimit for SupplyLimitContract {
    #[only_role(operator, "manager")]
    fn set_supply_limit(e: &Env, token: Address, limit: i128, operator: Address) {
        supply_limit::set_supply_limit(e, &token, limit);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for SupplyLimitContract {
    // Transfers do not affect the tracked supply; no auth or bookkeeping
    // needed.
    fn on_transfer(
        _e: &Env,
        _from: AccountSnapshot,
        _to: AccountSnapshot,
        _amount: i128,
        _spender: Option<Address>,
        _token: Address,
    ) {
    }

    fn on_created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        supply_limit::on_created(e, &to.address, amount, &token);
    }

    fn on_destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        supply_limit::on_destroyed(e, &from.address, amount, &token);
    }

    fn can_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        _spender: Option<Address>,
        token: Address,
    ) -> bool {
        supply_limit::can_transfer(e, &from.address, &to.address, amount, &token)
    }

    fn can_create(e: &Env, to: AccountSnapshot, amount: i128, token: Address) -> bool {
        supply_limit::can_create(e, &to.address, amount, &token)
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "SupplyLimitModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
