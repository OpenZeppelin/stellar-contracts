use soroban_sdk::{Address, Env};
use stellar_pausable::{Pausable, PausableExt};

use crate::{extensions::burnable::FungibleBurnable, fungible::FungibleToken};

impl<T: Pausable, N: FungibleToken> FungibleToken for PausableExt<T, N> {
    type Impl = N;

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        T::when_not_paused(e);
        Self::Impl::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        T::when_not_paused(e);
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }
}

impl<T: Pausable, N: FungibleBurnable> FungibleBurnable for PausableExt<T, N> {
    type Impl = N;

    fn burn(e: &Env, from: &Address, amount: i128) {
        T::when_not_paused(e);
        Self::Impl::burn(e, from, amount)
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        T::when_not_paused(e);
        Self::Impl::burn_from(e, spender, from, amount)
    }
}
