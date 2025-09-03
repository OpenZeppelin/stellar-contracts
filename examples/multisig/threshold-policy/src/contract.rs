use soroban_sdk::{auth::Context, contract, contractimpl, Address, Env, Vec};
use stellar_accounts::{
    policies::{simple_threshold, Policy},
    smart_account::Signer,
};

#[contract]
pub struct ThresholdPolicyContract;

#[contractimpl]
impl Policy for ThresholdPolicyContract {
    type InstallParams = simple_threshold::SimpleThresholdInstallParams;

    fn can_enforce(
        e: &Env,
        context: Context,
        context_rule_signers: Vec<Signer>,
        authenticated_signers: Vec<Signer>,
        smart_account: Address,
    ) -> bool {
        simple_threshold::can_enforce(
            e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        )
    }

    fn enforce(
        e: &Env,
        context: Context,
        context_rule_signers: Vec<Signer>,
        authenticated_signers: Vec<Signer>,
        smart_account: Address,
    ) {
        simple_threshold::enforce(
            e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        )
    }

    fn install(e: &Env, install_params: Self::InstallParams, smart_account: Address) {
        simple_threshold::install(e, &install_params, &smart_account)
    }

    fn uninstall(e: &Env, smart_account: Address) {
        simple_threshold::uninstall(e, &smart_account)
    }
}

#[contractimpl]
impl ThresholdPolicyContract {
    /// Get the current threshold for a smart account
    pub fn get_threshold(e: Env, smart_account: Address) -> u32 {
        simple_threshold::get_threshold(&e, &smart_account)
    }

    /// Set a new threshold for a smart account
    pub fn set_threshold(e: Env, threshold: u32, signers_count: u32, smart_account: Address) {
        simple_threshold::set_threshold(&e, signers_count, threshold, &smart_account)
    }
}
