// A conditional attribute that applies `no_std` only for wasm targets.
// This prevents Cargo from implicitly injecting std::prelude imports into empty
// crates when building for wasm targets that don't support std (like
// wasm32v1-none).
#![cfg_attr(target_family = "wasm", no_std)]

// The entire implementation is only compiled for non-wasm targets.
// This is a test utility crate that's not needed in wasm environments.
#[cfg(not(target_family = "wasm"))]
mod implementation {
    use std::collections::HashSet;

    use soroban_sdk::{
        symbol_short, testutils::Events, xdr, Address, Env, FromVal, IntoVal, Map, Symbol,
        TryFromVal, Val,
    };

    pub struct EventAssertion<'a> {
        env: &'a Env,
        contract: Address,
        processed_events: HashSet<u32>,
    }

    impl<'a> EventAssertion<'a> {
        pub fn new(env: &'a Env, contract: Address) -> Self {
            Self { env, contract, processed_events: HashSet::new() }
        }

        fn expect_event(&mut self, symbol_name: &str, missing_message: &str) -> xdr::ContractEvent {
            self.find_event_by_symbol(symbol_name).expect(missing_message)
        }

        fn assert_event_contract(&self, event: &xdr::ContractEvent) {
            assert_eq!(event.contract_id, Some(self.contract_id()), "Event from wrong contract");
        }

        fn contract_id(&self) -> xdr::ContractId {
            match xdr::ScAddress::from(&self.contract) {
                xdr::ScAddress::Contract(contract_id) => contract_id,
                _ => panic!("expected contract address"),
            }
        }

        fn event_topics_data<'b>(
            &self,
            event: &'b xdr::ContractEvent,
        ) -> (&'b xdr::VecM<xdr::ScVal>, &'b xdr::ScVal) {
            match &event.body {
                xdr::ContractEventBody::V0(body) => (&body.topics, &body.data),
            }
        }

        fn topic_as<T: FromVal<Env, xdr::ScVal>>(
            &self,
            topics: &xdr::VecM<xdr::ScVal>,
            index: usize,
        ) -> T {
            T::from_val(self.env, topics.get(index).unwrap())
        }

        fn topic_as_val<T: FromVal<Env, Val>>(
            &self,
            topics: &xdr::VecM<xdr::ScVal>,
            index: usize,
        ) -> T {
            let val = Val::try_from_val(self.env, topics.get(index).unwrap()).unwrap();
            T::from_val(self.env, &val)
        }

        fn assert_topic_symbol(
            &self,
            topics: &xdr::VecM<xdr::ScVal>,
            index: usize,
            expected: Symbol,
        ) {
            let topic_symbol: Symbol = self.topic_as(topics, index);
            assert_eq!(topic_symbol, expected);
        }

        fn find_event_by_symbol(&mut self, symbol_name: &str) -> Option<xdr::ContractEvent> {
            let events = self.env.events().all();

            let target_symbol = match symbol_name {
                "transfer" => symbol_short!("transfer"),
                "mint" => symbol_short!("mint"),
                "burn" => symbol_short!("burn"),
                "approve" => symbol_short!("approve"),
                _ => Symbol::new(self.env, symbol_name),
            };

            for (index, event) in events.events().iter().enumerate() {
                let index_u32 = index as u32;

                if self.processed_events.contains(&index_u32) {
                    continue;
                }

                let (topics, _data) = self.event_topics_data(event);

                if let Some(first_topic) = topics.first() {
                    let topic_symbol: Symbol = Symbol::from_val(self.env, first_topic);

                    if topic_symbol == target_symbol {
                        self.processed_events.insert(index_u32);
                        return Some(event.clone());
                    }
                }
            }
            None
        }

        pub fn assert_fungible_transfer(
            &mut self,
            from: &Address,
            to: &Address,
            muxed_id: Option<u64>,
            amount: i128,
        ) {
            let event = self.expect_event("transfer", "Transfer event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 3, "Transfer event should have 3 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("transfer"));

            let event_from: Address = self.topic_as(topics, 1);
            let event_to: Address = self.topic_as(topics, 2);

            let data_map: Map<Symbol, Val> = Map::from_val(self.env, data);
            let event_amount: i128 =
                data_map.get(symbol_short!("amount")).unwrap().into_val(self.env);
            let event_muxed_id: Option<u64> =
                data_map.get(Symbol::new(self.env, "to_muxed_id")).unwrap().into_val(self.env);

            assert_eq!(&event_from, from, "Transfer event has wrong from address");
            assert_eq!(&event_to, to, "Transfer event has wrong to address");
            assert_eq!(event_amount, amount, "Transfer event has wrong amount");
            assert_eq!(event_muxed_id, muxed_id, "Transfer event has wrong muxed id");
        }

        pub fn assert_non_fungible_transfer(
            &mut self,
            from: &Address,
            to: &Address,
            token_id: u32,
        ) {
            let event = self.expect_event("transfer", "Transfer event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 3, "Transfer event should have 3 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("transfer"));

            let event_from: Address = self.topic_as(topics, 1);
            let event_to: Address = self.topic_as(topics, 2);

            let data_map: Map<Symbol, u32> = Map::from_val(self.env, data);
            let event_token_id = data_map.get(Symbol::new(self.env, "token_id")).unwrap();

            assert_eq!(&event_from, from, "Transfer event has wrong from address");
            assert_eq!(&event_to, to, "Transfer event has wrong to address");
            assert_eq!(event_token_id, token_id, "Transfer event has wrong amount");
        }

        pub fn assert_fungible_mint(&mut self, to: &Address, amount: i128) {
            let event = self.expect_event("mint", "Mint event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 2, "Mint event should have 2 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("mint"));

            let event_to: Address = self.topic_as(topics, 1);

            let data_map: Map<Symbol, i128> = Map::from_val(self.env, data);
            let event_amount = data_map.get(Symbol::new(self.env, "amount")).unwrap();

            assert_eq!(&event_to, to, "Mint event has wrong to address");
            assert_eq!(event_amount, amount, "Mint event has wrong amount");
        }

        pub fn assert_non_fungible_mint(&mut self, to: &Address, token_id: u32) {
            let event = self.expect_event("mint", "Mint event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 2, "Mint event should have 2 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("mint"));

            let event_to: Address = self.topic_as(topics, 1);

            let data_map: Map<Symbol, u32> = Map::from_val(self.env, data);
            let event_token_id = data_map.get(Symbol::new(self.env, "token_id")).unwrap();

            assert_eq!(&event_to, to, "Mint event has wrong to address");
            assert_eq!(event_token_id, token_id, "Mint event has wrong token_id");
        }

        pub fn assert_fungible_burn(&mut self, from: &Address, amount: i128) {
            let event = self.expect_event("burn", "Burn event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 2, "Burn event should have 2 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("burn"));

            let event_from: Address = self.topic_as(topics, 1);

            let data_map: Map<Symbol, i128> = Map::from_val(self.env, data);
            let event_amount = data_map.get(Symbol::new(self.env, "amount")).unwrap();

            assert_eq!(&event_from, from, "Burn event has wrong from address");
            assert_eq!(event_amount, amount, "Burn event has wrong amount");
        }

        pub fn assert_non_fungible_burn(&mut self, from: &Address, token_id: u32) {
            let event = self.expect_event("burn", "Burn event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 2, "Burn event should have 2 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("burn"));

            let event_from: Address = self.topic_as(topics, 1);

            let data_map: Map<Symbol, u32> = Map::from_val(self.env, data);
            let event_token_id = data_map.get(Symbol::new(self.env, "token_id")).unwrap();

            assert_eq!(&event_from, from, "Burn event has wrong from address");
            assert_eq!(event_token_id, token_id, "Burn event has wrong token_id");
        }

        pub fn assert_event_count(&self, expected: usize) {
            let events = self.env.events().all();
            let event_count = events.events().len();
            assert_eq!(
                event_count, expected,
                "Expected {} events, found {}",
                expected, event_count
            );
        }

        pub fn assert_fungible_approve(
            &mut self,
            owner: &Address,
            spender: &Address,
            amount: i128,
            live_until_ledger: u32,
        ) {
            let event = self.expect_event("approve", "Approve event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 3, "Approve event should have 3 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("approve"));

            let event_owner: Address = self.topic_as(topics, 1);
            let event_spender: Address = self.topic_as(topics, 2);

            let data_map: Map<Symbol, Val> = Map::from_val(self.env, data);
            let event_amount: i128 =
                data_map.get(Symbol::new(self.env, "amount")).unwrap().into_val(self.env);
            let event_live_until_ledger: u32 = data_map
                .get(Symbol::new(self.env, "live_until_ledger"))
                .unwrap()
                .into_val(self.env);

            assert_eq!(&event_owner, owner, "Approve event has wrong owner address");
            assert_eq!(&event_spender, spender, "Approve event has wrong spender address");
            assert_eq!(event_amount, amount, "Approve event has wrong amount");
            assert_eq!(
                event_live_until_ledger, live_until_ledger,
                "Approve event has wrong live_until_ledger"
            );
        }

        pub fn assert_non_fungible_approve(
            &mut self,
            owner: &Address,
            spender: &Address,
            token_id: u32,
            live_until_ledger: u32,
        ) {
            let event = self.expect_event("approve", "Approve event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 3, "Approve event should have 3 topics");

            self.assert_topic_symbol(topics, 0, symbol_short!("approve"));

            let event_owner: Address = self.topic_as(topics, 1);
            let event_token_id: u32 = self.topic_as_val(topics, 2);

            let data_map: Map<Symbol, Val> = Map::from_val(self.env, data);
            let event_approved: Address =
                data_map.get(Symbol::new(self.env, "approved")).unwrap().into_val(self.env);
            let event_live_until_ledger: u32 = data_map
                .get(Symbol::new(self.env, "live_until_ledger"))
                .unwrap()
                .into_val(self.env);

            assert_eq!(&event_owner, owner, "Approve event has wrong owner address");
            assert_eq!(event_token_id, token_id, "Approve event has wrong token_id");
            assert_eq!(event_approved, *spender, "Approve event has wrong approved address");
            assert_eq!(
                event_live_until_ledger, live_until_ledger,
                "Approve event has wrong live_until_ledger"
            );
        }

        pub fn assert_approve_for_all(
            &mut self,
            owner: &Address,
            operator: &Address,
            live_until_ledger: u32,
        ) {
            let event =
                self.expect_event("approve_for_all", "ApproveForAll event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 2, "ApproveForAll event should have 2 topics");

            self.assert_topic_symbol(topics, 0, Symbol::new(self.env, "approve_for_all"));

            let event_owner: Address = self.topic_as(topics, 1);

            let data_map: Map<Symbol, Val> = Map::from_val(self.env, data);
            let event_operator: Address =
                data_map.get(Symbol::new(self.env, "operator")).unwrap().into_val(self.env);
            let event_live_until_ledger: u32 = data_map
                .get(Symbol::new(self.env, "live_until_ledger"))
                .unwrap()
                .into_val(self.env);

            assert_eq!(&event_owner, owner, "ApproveForAll event has wrong owner address");
            assert_eq!(event_operator, *operator, "ApproveForAll event has wrong operator address");
            assert_eq!(
                event_live_until_ledger, live_until_ledger,
                "ApproveForAll event has wrong live_until_ledger"
            );
        }

        pub fn assert_consecutive_mint(&mut self, to: &Address, from_id: u32, to_id: u32) {
            let event = self
                .expect_event("consecutive_mint", "ConsecutiveMint event not found in event log");
            self.assert_event_contract(&event);

            let (topics, data) = self.event_topics_data(&event);
            assert_eq!(topics.len(), 2, "ConsecutiveMint event should have 2 topics");

            self.assert_topic_symbol(topics, 0, Symbol::new(self.env, "consecutive_mint"));

            let event_to: Address = self.topic_as(topics, 1);

            let data_map: Map<Symbol, u32> = Map::from_val(self.env, data);
            let from_token_id_val = data_map.get(Symbol::new(self.env, "from_token_id")).unwrap();
            let to_token_id_val = data_map.get(Symbol::new(self.env, "to_token_id")).unwrap();

            assert_eq!(&event_to, to, "ConsecutiveMint event has wrong to address");
            assert_eq!(from_token_id_val, from_id, "ConsecutiveMint event has wrong from_token_id");
            assert_eq!(to_token_id_val, to_id, "ConsecutiveMint event has wrong to_token_id");
        }
    }
}

// Re-export for non-wasm targets
#[cfg(not(target_family = "wasm"))]
pub use implementation::*;
