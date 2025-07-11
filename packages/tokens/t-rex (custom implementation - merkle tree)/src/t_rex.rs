use soroban_sdk::{Address, Env, String, Vec};
use stellar_fungible::FungibleToken;

// # Draft Notes
//
// 1. It turns out we made a right choice to have burnable as a separate trait,
// `FungibleToken` plays nicely here, because the T-Rex interface doesn't have `burn_from`
// 2. Added `agent` to setter functions so that the calls can be authorized.
pub trait TRexToken: FungibleToken {
    // T-REX GETTER FUNCTIONS
    fn version(e: &Env) -> String;

    fn merkle_tree(e: &Env) -> Address;

    fn compliance(e: &Env) -> Address;

    fn is_frozen(e: &Env, account: Address) -> bool;

    fn get_frozen_token(e: &Env, account: Address) -> i128;

    // T-REX SETTER FUNCTIONS

    fn set_name(e: &Env, name: String, agent: Address);

    fn set_symbol(e: &Env, symbol: String, agent: Address);

    fn pause(e: &Env, agent: Address);

    fn unpause(e: &Env, agent: Address);

    fn set_address_frozen(e: &Env, account: Address, freeze: bool, agent: Address);

    fn freeze_partial_tokens(e: &Env, account: Address, amount: i128, agent: Address);

    fn unfreeze_partial_tokens(e: &Env, account: Address, amount: i128, agent: Address);

    fn set_merkle_tree(e: &Env, merkle_tree: Address, agent: Address);

    fn set_compliance(e: &Env, compliance: Address, agent: Address);

    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, agent: Address) -> bool;

    fn mint(e: &Env, to: Address, amount: i128, agent: Address);

    // burning here can be done only by an Agent
    fn burn(e: &Env, account: Address, amount: i128, agent: Address);

    fn recovery_address(
        e: &Env,
        lost_wallet: Address,
        new_wallet: Address,
        investor_onchain_id: Address,
        agent: Address,
    ) -> bool;

    // T-REX BATCH SETTER FUNCTIONS

    fn batch_transfer(
        e: &Env,
        from: Address,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
        agent: Address,
    );

    fn batch_forced_transfer(
        e: &Env,
        from_list: Vec<Address>,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
        agent: Address,
    );

    fn batch_mint(e: &Env, to_list: Vec<Address>, amounts: Vec<i128>, agent: Address);

    // burning here can be done only by an Agent
    fn batch_burn(e: &Env, account_addresses: Vec<Address>, amounts: Vec<i128>, agent: Address);

    fn batch_set_address_frozen(
        e: &Env,
        account_addresses: Vec<Address>,
        freeze: Vec<bool>,
        agent: Address,
    );

    fn batch_freeze_partial_tokens(
        e: &Env,
        account_addresses: Vec<Address>,
        amounts: Vec<i128>,
        agent: Address,
    );

    fn batch_unfreeze_partial_tokens(
        e: &Env,
        account_addresses: Vec<Address>,
        amounts: Vec<i128>,
        agent: Address,
    );
}
