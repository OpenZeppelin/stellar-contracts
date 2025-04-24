#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashSet;

use soroban_sdk::{symbol_short, testutils::Events, Address, Env, IntoVal, Symbol, Val, Vec};
use stellar_non_fungible::TokenId;

pub struct EventAssertion<'a> {
    env: &'a Env,
    contract: Address,
    processed_events: HashSet<u32>,
}

impl<'a> EventAssertion<'a> {
    pub fn new(env: &'a Env, contract: Address) -> Self {
        Self { env, contract, processed_events: HashSet::new() }
    }

    fn find_event_by_symbol(&mut self, symbol_name: &str) -> Option<(Address, Vec<Val>, Val)> {
        let events = self.env.events().all();

        let target_symbol = match symbol_name {
            "transfer" => symbol_short!("transfer"),
            "mint" => symbol_short!("mint"),
            "burn" => symbol_short!("burn"),
            "approve" => symbol_short!("approve"),
            _ => Symbol::new(self.env, symbol_name),
        };

        for (index, event) in events.iter().enumerate() {
            let index_u32 = index as u32;

            if self.processed_events.contains(&index_u32) {
                continue;
            }

            let (contract, topics, data) = event;

            let topics_clone = topics.clone();

            if let Some(first_topic) = topics_clone.first() {
                let topic_symbol: Symbol = first_topic.into_val(self.env);

                if topic_symbol == target_symbol {
                    self.processed_events.insert(index_u32);
                    return Some((contract.clone(), topics_clone, data));
                }
            }
        }
        None
    }

    pub fn assert_fungible_transfer(&mut self, from: &Address, to: &Address, amount: i128) {
        let transfer_event = self.find_event_by_symbol("transfer");

        assert!(transfer_event.is_some(), "Transfer event not found in event log");

        let (contract, topics, data) = transfer_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 3, "Transfer event should have 3 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("transfer"));

        let event_from: Address = topics.get_unchecked(1).into_val(self.env);
        let event_to: Address = topics.get_unchecked(2).into_val(self.env);
        let event_amount: i128 = data.into_val(self.env);

        assert_eq!(&event_from, from, "Transfer event has wrong from address");
        assert_eq!(&event_to, to, "Transfer event has wrong to address");
        assert_eq!(event_amount, amount, "Transfer event has wrong amount");
    }

    pub fn assert_non_fungible_transfer(
        &mut self,
        from: &Address,
        to: &Address,
        token_id: TokenId,
    ) {
        let transfer_event = self.find_event_by_symbol("transfer");

        assert!(transfer_event.is_some(), "Transfer event not found in event log");

        let (contract, topics, data) = transfer_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 3, "Transfer event should have 3 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("transfer"));

        let event_from: Address = topics.get_unchecked(1).into_val(self.env);
        let event_to: Address = topics.get_unchecked(2).into_val(self.env);
        let event_token_id: TokenId = data.into_val(self.env);

        assert_eq!(&event_from, from, "Transfer event has wrong from address");
        assert_eq!(&event_to, to, "Transfer event has wrong to address");
        assert_eq!(event_token_id, token_id, "Transfer event has wrong amount");
    }

    pub fn assert_fungible_mint(&mut self, to: &Address, amount: i128) {
        let mint_event = self.find_event_by_symbol("mint");

        assert!(mint_event.is_some(), "Mint event not found in event log");

        let (contract, topics, data) = mint_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "Mint event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("mint"));

        let event_to: Address = topics.get_unchecked(1).into_val(self.env);
        let event_amount: i128 = data.into_val(self.env);

        assert_eq!(&event_to, to, "Mint event has wrong to address");
        assert_eq!(event_amount, amount, "Mint event has wrong amount");
    }

    pub fn assert_non_fungible_mint(&mut self, to: &Address, token_id: TokenId) {
        let mint_event = self.find_event_by_symbol("mint");

        assert!(mint_event.is_some(), "Mint event not found in event log");

        let (contract, topics, data) = mint_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "Mint event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("mint"));

        let event_to: Address = topics.get_unchecked(1).into_val(self.env);
        let event_token_id: TokenId = data.into_val(self.env);

        assert_eq!(&event_to, to, "Mint event has wrong to address");
        assert_eq!(event_token_id, token_id, "Mint event has wrong token_id");
    }

    pub fn assert_fungible_burn(&mut self, from: &Address, amount: i128) {
        let burn_event = self.find_event_by_symbol("burn");

        assert!(burn_event.is_some(), "Burn event not found in event log");

        let (contract, topics, data) = burn_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "Burn event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("burn"));

        let event_from: Address = topics.get_unchecked(1).into_val(self.env);
        let event_amount: i128 = data.into_val(self.env);

        assert_eq!(&event_from, from, "Burn event has wrong from address");
        assert_eq!(event_amount, amount, "Burn event has wrong amount");
    }

    pub fn assert_non_fungible_burn(&mut self, from: &Address, token_id: TokenId) {
        let burn_event = self.find_event_by_symbol("burn");

        assert!(burn_event.is_some(), "Burn event not found in event log");

        let (contract, topics, data) = burn_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "Burn event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("burn"));

        let event_from: Address = topics.get_unchecked(1).into_val(self.env);
        let event_token_id: TokenId = data.into_val(self.env);

        assert_eq!(&event_from, from, "Burn event has wrong from address");
        assert_eq!(event_token_id, token_id, "Burn event has wrong token_id");
    }

    pub fn assert_event_count(&self, expected: usize) {
        let events = self.env.events().all();
        assert_eq!(
            events.len() as usize,
            expected,
            "Expected {} events, found {}",
            expected,
            events.len()
        );
    }

    pub fn assert_fungible_approve(
        &mut self,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        let approve_event = self.find_event_by_symbol("approve");

        assert!(approve_event.is_some(), "Approve event not found in event log");

        let (contract, topics, data) = approve_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 3, "Approve event should have 3 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("approve"));

        let event_owner: Address = topics.get_unchecked(1).into_val(self.env);
        let event_spender: Address = topics.get_unchecked(2).into_val(self.env);
        let event_data: (i128, u32) = data.into_val(self.env);

        assert_eq!(&event_owner, owner, "Approve event has wrong owner address");
        assert_eq!(&event_spender, spender, "Approve event has wrong spender address");
        assert_eq!(event_data.0, amount, "Approve event has wrong amount");
        assert_eq!(event_data.1, live_until_ledger, "Approve event has wrong live_until_ledger");
    }

    pub fn assert_non_fungible_approve(
        &mut self,
        owner: &Address,
        spender: &Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        let approve_event = self.find_event_by_symbol("approve");

        assert!(approve_event.is_some(), "Approve event not found in event log");

        let (contract, topics, data) = approve_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 3, "Approve event should have 3 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("approve"));

        let event_owner: Address = topics.get_unchecked(1).into_val(self.env);
        let event_token_id: TokenId = topics.get_unchecked(2).into_val(self.env);
        let event_data: (Address, u32) = data.into_val(self.env);

        assert_eq!(&event_owner, owner, "Approve event has wrong owner address");
        assert_eq!(event_token_id, token_id, "Approve event has wrong spender address");
        assert_eq!(event_data.0, *spender, "Approve event has wrong token_id");
        assert_eq!(event_data.1, live_until_ledger, "Approve event has wrong live_until_ledger");
    }

    pub fn assert_approve_for_all(
        &mut self,
        owner: &Address,
        operator: &Address,
        live_until_ledger: u32,
    ) {
        let approve_event = self.find_event_by_symbol("approve_for_all");

        assert!(approve_event.is_some(), "ApproveForAll event not found in event log");

        let (contract, topics, data) = approve_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "ApproveForAll event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, Symbol::new(self.env, "approve_for_all"));

        let event_owner: Address = topics.get_unchecked(1).into_val(self.env);
        let event_data: (Address, u32) = data.into_val(self.env);

        assert_eq!(&event_owner, owner, "Approve event has wrong owner address");
        assert_eq!(event_data.0, *operator, "Approve event has wrong operator address");
        assert_eq!(event_data.1, live_until_ledger, "Approve event has wrong live_until_ledger");
    }

    pub fn assert_consecutive_mint(&mut self, to: &Address, from_id: TokenId, to_id: TokenId) {
        let event = self.find_event_by_symbol("consecutive_mint");

        assert!(event.is_some(), "ConsecutiveMint event not found in event log");

        let (contract, topics, data) = event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "ConsecutiveMint event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, Symbol::new(self.env, "consecutive_mint"));

        let event_to: Address = topics.get_unchecked(1).into_val(self.env);
        let event_data: (TokenId, TokenId) = data.into_val(self.env);

        assert_eq!(&event_to, to, "ConsecutiveMint event has wrong to address");
        assert_eq!(event_data.0, from_id, "ConsecutiveMint event has wrong from_token_id");
        assert_eq!(event_data.1, to_id, "ConsecutiveMint event has wrong to_token_id");
    }
}
