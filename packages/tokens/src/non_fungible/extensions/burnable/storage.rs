use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    burnable::NonFungibleBurnable, extensions::burnable::emit_burn, NFTBase,
};

impl NonFungibleBurnable for NFTBase {
    type Impl = Self;

    fn burn(e: &Env, from: &Address, token_id: u32) {
        from.require_auth();
        Self::update(e, Some(from), None, token_id);
        emit_burn(e, from, token_id);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        spender.require_auth();
        Self::check_spender_approval(e, spender, from, token_id);
        Self::update(e, Some(from), None, token_id);
        emit_burn(e, from, token_id);
    }
}
