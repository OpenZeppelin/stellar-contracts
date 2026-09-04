//! RWA Token Example Contract.

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, Env, MuxedAddress, String,
    Symbol, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    rwa::{RWAError, RWAToken, RWA},
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct RWATokenContract;

#[contractimpl]
impl RWATokenContract {
    pub fn __constructor(
        e: &Env,
        name: String,
        symbol: String,
        admin: Address,
        manager: Address,
        compliance: Address,
        identity_verifier: Address,
    ) {
        Base::set_metadata(e, 7, name, symbol);

        access_control::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);

        RWA::set_compliance(e, &compliance);
        RWA::set_identity_verifier(e, &identity_verifier);
    }
}

#[contractimpl(contracttrait)]
impl Pausable for RWATokenContract {
    #[only_admin]
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    #[only_admin]
    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for RWATokenContract {
    type ContractType = RWA;

    /// Showcase: how to opt out of custodial (muxed) destinations.
    ///
    /// **SHOWCASE ONLY**: this override exists purely to illustrate how the
    /// opt-out is written. It is neither a recommendation nor part of the
    /// library's behavior, and it is not required for a conforming RWA token.
    /// Whether to accept custodial destinations is entirely the implementor's
    /// decision, and both answers are legitimate.
    ///
    /// Background: [`RWA`] accepts a muxed destination. Identity verification
    /// and compliance run against the base address, and the muxed ID is
    /// recorded in the transfer event so a custodian can attribute the
    /// transfer to one of its off-chain sub-accounts. A verified holder may
    /// therefore be a custodian holding on behalf of beneficiaries who have no
    /// on-chain identity of their own, and the ID itself carries no on-chain
    /// verification. That is the intended arrangement for omnibus custody, and
    /// it is what the library does by default.
    ///
    /// An issuer that instead requires beneficial owners on the on-chain
    /// register can refuse muxed destinations, which is what the body below
    /// does. An issuer content with omnibus custody should simply delete this
    /// override; the library default then applies and muxed destinations are
    /// accepted.
    fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
        if to.id().is_some() {
            panic_with_error!(e, RWAError::IdentityVerificationFailed);
        }
        RWA::transfer(e, &from, &to, amount);
    }
}

#[contractimpl(contracttrait)]
impl RWAToken for RWATokenContract {
    #[only_role(operator, "manager")]
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address) {
        RWA::forced_transfer(e, &from, &to, amount);
    }

    #[only_role(operator, "manager")]
    fn mint(e: &Env, to: Address, amount: i128, operator: Address) {
        RWA::mint(e, &to, amount);
    }

    #[only_role(operator, "manager")]
    fn burn(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::burn(e, &user_address, amount);
    }

    #[only_role(operator, "manager")]
    fn recover_balance(
        e: &Env,
        old_account: Address,
        new_account: Address,
        operator: Address,
    ) -> bool {
        RWA::recover_balance(e, &old_account, &new_account)
    }

    #[only_role(operator, "manager")]
    fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address) {
        RWA::set_address_frozen(e, &user_address, freeze);
    }

    #[only_role(operator, "manager")]
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::freeze_partial_tokens(e, &user_address, amount);
    }

    #[only_role(operator, "manager")]
    fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::unfreeze_partial_tokens(e, &user_address, amount);
    }

    #[only_role(operator, "manager")]
    fn set_compliance(e: &Env, compliance: Address, operator: Address) {
        RWA::set_compliance(e, &compliance);
    }

    #[only_role(operator, "manager")]
    fn set_identity_verifier(e: &Env, identity_verifier: Address, operator: Address) {
        RWA::set_identity_verifier(e, &identity_verifier);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for RWATokenContract {}
