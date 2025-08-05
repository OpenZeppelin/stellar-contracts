use crate::Error;
use soroban_sdk::*;

pub mod keys;

pub trait DiamondProxyExecutor {
    fn get_env(&self) -> &Env;
    fn init_contract(&self, contract_address: &Address, args: Vec<Val>) -> Result<(), Error> {
        self.get_env().invoke_contract::<Result<(), Error>>(
            contract_address,
            &Symbol::new(self.get_env(), "init"),
            args,
        )
    }

    /// Executes a facet function via the diamond proxy's fallback implementation
    fn facet_execute<T>(
        &self,
        diamond_proxy_address: &Address,
        function_name: &Symbol,
        parameters: Vec<Val>,
    ) -> Result<T, Error>
    where
        T: TryFromVal<Env, Val>,
    {
        let result_fallback = self.get_env().invoke_contract::<Result<Val, Error>>(
            diamond_proxy_address,
            &Symbol::new(self.get_env(), "fallback"),
            soroban_sdk::vec![
                &self.get_env(),
                function_name.into_val(self.get_env()),
                parameters.into_val(self.get_env())
            ],
        );

        match result_fallback?.try_into_val(self.get_env()) {
            Ok(val) => Ok(val),
            Err(err) => {
                panic!("FAILURE IN CONVERSION: {err:?}");
            }
        }
    }
}

impl DiamondProxyExecutor for Env {
    fn get_env(&self) -> &Env {
        self
    }
}
