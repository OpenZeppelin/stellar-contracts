/// For contracts that implement `FungibleToken` and `FungibleBurnable`,
/// the implementation of `TokenInterface` can be generated using the
/// `impl_token_interface!` macro.
#[macro_export]
macro_rules! impl_token_interface {
    ($contract:ty) => {
        impl soroban_sdk::token::TokenInterface for $contract
        where
            $contract:
                stellar_fungible::FungibleToken + stellar_fungible::burnable::FungibleBurnable,
        {
            fn balance(env: soroban_sdk::Env, id: soroban_sdk::Address) -> i128 {
                <Self as FungibleToken>::ContractType::balance(&env, &id)
            }

            fn transfer(
                env: soroban_sdk::Env,
                from: soroban_sdk::Address,
                to: soroban_sdk::Address,
                amount: i128,
            ) {
                <Self as FungibleToken>::ContractType::transfer(&env, &from, &to, amount)
            }

            fn transfer_from(
                env: soroban_sdk::Env,
                spender: soroban_sdk::Address,
                from: soroban_sdk::Address,
                to: soroban_sdk::Address,
                amount: i128,
            ) {
                <Self as FungibleToken>::ContractType::transfer_from(
                    &env, &spender, &from, &to, amount,
                )
            }

            fn burn(env: soroban_sdk::Env, from: soroban_sdk::Address, amount: i128) {
                <Self as FungibleToken>::ContractType::burn(&env, &from, amount)
            }

            fn burn_from(
                env: soroban_sdk::Env,
                spender: soroban_sdk::Address,
                from: soroban_sdk::Address,
                amount: i128,
            ) {
                <Self as FungibleToken>::ContractType::burn_from(&env, &spender, &from, amount)
            }

            fn allowance(
                env: soroban_sdk::Env,
                from: soroban_sdk::Address,
                spender: soroban_sdk::Address,
            ) -> i128 {
                <Self as FungibleToken>::ContractType::allowance(&env, &from, &spender)
            }

            fn approve(
                env: soroban_sdk::Env,
                from: soroban_sdk::Address,
                spender: soroban_sdk::Address,
                amount: i128,
                live_until_ledger: u32,
            ) {
                <Self as FungibleToken>::ContractType::approve(
                    &env,
                    &from,
                    &spender,
                    amount,
                    live_until_ledger,
                )
            }

            fn decimals(env: soroban_sdk::Env) -> u32 {
                <Self as FungibleToken>::ContractType::decimals(&env)
            }

            fn name(env: soroban_sdk::Env) -> soroban_sdk::String {
                <Self as FungibleToken>::ContractType::name(&env)
            }

            fn symbol(env: soroban_sdk::Env) -> soroban_sdk::String {
                <Self as FungibleToken>::ContractType::symbol(&env)
            }
        }
    };
}
