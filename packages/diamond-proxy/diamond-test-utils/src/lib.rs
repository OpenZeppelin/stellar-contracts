use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, IntoVal, Symbol, Val, Vec,
};
use stellar_diamond_proxy_core::facets::{FacetAction, FacetCut};

// Re-export commonly used types
pub use stellar_diamond_proxy_core::facets::{
    FacetAction as TestFacetAction, FacetCut as TestFacetCut,
};

/// Helper struct to manage diamond proxy testing
pub struct DiamondTestUtils {
    pub env: Env,
    pub diamond_id: Address,
    pub owner: Address,
    // Deployed facets go here
    pub deployed_facets: Vec<Address>,
}

impl DiamondTestUtils {
    /// Create a new diamond proxy instance for testing
    pub fn new(env: &Env, diamond_proxy_wasm: &[u8]) -> Self {
        let diamond_id = env.register(diamond_proxy_wasm, ());
        let owner = Address::generate(env);
        env.mock_all_auths_allowing_non_root_auth();

        // Initialize the diamond proxy
        let salt = BytesN::<32>::random(env);
        let init_args = vec![env, owner.clone().into_val(env), salt.into_val(env)];

        let _ = env.invoke_contract::<Val>(&diamond_id, &Symbol::new(env, "init"), init_args);

        Self {
            env: env.clone(),
            diamond_id,
            owner,
            deployed_facets: Vec::new(env),
        }
    }

    /// Deploy a facet and add it to the diamond
    pub fn add_facet(&mut self, facet_wasm: &[u8], selectors: Vec<Symbol>) -> Address {
        let wasm_hash = self.env.deployer().upload_contract_wasm(facet_wasm);
        let salt = BytesN::<32>::random(&self.env);

        let cut = FacetCut {
            action: FacetAction::Add,
            selectors: selectors.clone(),
            wasm_hash_of_facet: wasm_hash,
            salt,
        };

        let cuts = vec![&self.env, cut];
        let result: Vec<Address> = self.env.invoke_contract(
            &self.diamond_id,
            &Symbol::new(&self.env, "diamond_cut"),
            vec![&self.env, cuts.into_val(&self.env)],
        );

        let facet_addr = result.get(0).unwrap();
        self.deployed_facets.push_back(facet_addr.clone());
        facet_addr
    }

    /// Execute a function through the diamond proxy
    pub fn execute<T: soroban_sdk::TryFromVal<Env, Val>>(
        &self,
        selector: &str,
        args: Vec<Val>,
    ) -> T {
        self.env.invoke_contract(
            &self.diamond_id,
            &Symbol::new(&self.env, "fallback"),
            vec![
                &self.env,
                Symbol::new(&self.env, selector).into_val(&self.env),
                args.into_val(&self.env),
            ],
        )
    }

    /// Execute a function that might fail (returns None on error)
    pub fn try_execute<T: soroban_sdk::TryFromVal<Env, Val>>(
        &self,
        selector: &str,
        args: Vec<Val>,
    ) -> Option<T> {
        // Use a simple approach - catch panics and return None
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.execute(selector, args)
        }))
        .ok()
    }
}

/// Macro to simplify diamond test setup
#[macro_export]
macro_rules! setup_diamond_test {
    ($env:expr, $diamond_wasm:expr) => {{
        use diamond_test_utils::DiamondTestUtils;
        DiamondTestUtils::new($env, $diamond_wasm)
    }};
}

/// Macro to add multiple facets at once
#[macro_export]
macro_rules! add_facets {
    ($utils:expr, $( ($wasm:expr, [$($selector:expr),* $(,)?]) ),* $(,)? ) => {{
        $(
            $utils.add_facet(
                $wasm,
                vec![
                    &$utils.env,
                    $(Symbol::new(&$utils.env, $selector)),*
                ],
            );
        )*
    }};
}
