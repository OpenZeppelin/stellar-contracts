use soroban_sdk::{Address, Env, String, Vec};
use stellar_fungible::FungibleToken;

// # Draft Notes
//
// 1. It turns out we made a right choice to have burnable as a separate trait,
// `FungibleToken` plays nicely here, because the T-Rex interface doesn't have `burn_from`
// 2. Added `operator` to setter functions so that the calls can be authorized
pub trait TRexToken: FungibleToken {
    // T-REX GETTER FUNCTIONS
    fn version(e: &Env) -> String;

    fn onchain_id(e: &Env) -> Address;

    fn identity_registry(e: &Env) -> Address;

    fn compliance(e: &Env) -> Address;

    fn is_frozen(e: &Env, account: Address) -> bool;

    fn get_frozen_token(e: &Env, account: Address) -> i128;

    // T-REX SETTER FUNCTIONS

    fn set_name(e: &Env, operator: Address, name: String);

    fn set_symbol(e: &Env, operator: Address, symbol: String);

    fn set_onchain_id(e: &Env, operator: Address, onchain_id: Address);

    fn pause(e: &Env, operator: Address);

    fn unpause(e: &Env, operator: Address);

    fn set_address_frozen(e: &Env, operator: Address, account: Address, freeze: bool);

    fn freeze_partial_tokens(e: &Env, operator: Address, account: Address, amount: i128);

    fn unfreeze_partial_tokens(e: &Env, operator: Address, account: Address, amount: i128);

    fn set_identity_registry(e: &Env, operator: Address, identity_registry: Address);

    fn set_compliance(e: &Env, operator: Address, compliance: Address);

    fn forced_transfer(
        e: &Env,
        operator: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> bool;

    fn mint(e: &Env, operator: Address, to: Address, amount: i128);

    // burning here can be done only by an Agent
    fn burn(e: &Env, operator: Address, account: Address, amount: i128);

    fn recovery_address(
        e: &Env,
        operator: Address,
        lost_wallet: Address,
        new_wallet: Address,
        investor_onchain_id: Address,
    ) -> bool;

    // T-REX BATCH SETTER FUNCTIONS

    fn batch_transfer(e: &Env, from: Address, to_list: Vec<Address>, amounts: Vec<i128>);

    fn batch_forced_transfer(
        e: &Env,
        operator: Address,
        from_list: Vec<Address>,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
    );

    fn batch_mint(e: &Env, operator: Address, to_list: Vec<Address>, amounts: Vec<i128>);

    // burning here can be done only by an Agent
    fn batch_burn(e: &Env, operator: Address, user_addresses: Vec<Address>, amounts: Vec<i128>);

    fn batch_set_address_frozen(
        e: &Env,
        operator: Address,
        user_addresses: Vec<Address>,
        freeze: Vec<bool>,
    );

    fn batch_freeze_partial_tokens(
        e: &Env,
        operator: Address,
        user_addresses: Vec<Address>,
        amounts: Vec<i128>,
    );

    fn batch_unfreeze_partial_tokens(
        e: &Env,
        operator: Address,
        user_addresses: Vec<Address>,
        amounts: Vec<i128>,
    );
}
