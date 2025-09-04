use soroban_sdk::{auth::Context, contractclient, Address, Env, FromVal, Val, Vec};

use crate::smart_account::{ContextRule, Signer};

pub mod simple_threshold;
mod test;
pub mod weighted_threshold;

// can be shared across multiple smart accounts or owned by only one
pub trait Policy {
    type AccountParams: FromVal<Env, Val>;

    // only verify that policy is enforceable, it should trigger no state changes,
    // because can be called multiple times for different context rules
    // configurations
    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool;

    // this serves as a hook and can trigger state changes and must be authorized by
    // the smart account (`source.require_auth()`)
    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    );

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    );

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address);
}

// We need to declare a `PolicyClientInterface` here, instead of using the
// public trait above, because traits with associated types are not supported
// by the `#[contractclient]` macro. While this may appear redundant, it's a
// necessary workaround: we declare an identical internal trait with the macro
// to generate the required client implementation. Users should only interact
// with the public `Policy` trait above for their implementations.
#[allow(unused)]
#[contractclient(name = "PolicyClient")]
trait PolicyClientInterface {
    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool;

    // this serves as a hook and can trigger state changes and must be authorized by
    // the smart account (`source.require_auth()`)
    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    );

    fn install(e: &Env, install_params: Val, context_rule: ContextRule, smart_account: Address);

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address);
}
