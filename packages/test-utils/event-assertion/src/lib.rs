#![cfg(not(target_arch = "wasm32"))]

use soroban_sdk::{symbol_short, testutils::Events, Address, Env, IntoVal, Symbol, Val, Vec};
use stellar_non_fungible::TokenId;
use std::cell::RefCell;

pub struct EventAssertion<'a> {
    env: &'a Env,
    contract: Address,
    processed_events: RefCell<Vec<(u32, u32)>>,
    event_cache: RefCell<Option<Vec<(Address, Vec<Val>, Val)>>>,
}

impl<'a> EventAssertion<'a> {
    pub fn new(env: &'a Env, contract: Address) -> Self {
        Self { 
            env, 
            contract,
            processed_events: RefCell::new(Vec::new(env)),
            event_cache: RefCell::new(None),
        }
    }

    fn get_all_events(&self) -> Vec<(Address, Vec<Val>, Val)> {
        let mut cache = self.event_cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(self.env.events().all());
        }
        cache.clone().unwrap()
    }

    fn find_event_by_symbol(&self, symbol_name: &str) -> Option<(Address, Vec<Val>, Val)> {
        let events = self.get_all_events();
        let target_symbol = match symbol_name {
            "transfer" => symbol_short!("transfer"),
            "mint" => symbol_short!("mint"),
            "burn" => symbol_short!("burn"),
            "approve" => symbol_short!("approve"),
            _ => Symbol::new(self.env, symbol_name),
        };

        for (event_index, event) in events.iter().enumerate() {
            let topics: Vec<Val> = event.1.clone();
            if topics.is_empty() {
                continue;
            }
            
            let topic_symbol: Symbol = topics.first().unwrap().into_val(self.env);
            if topic_symbol == target_symbol {
                let event_key = (event_index as u32, 0);
                let mut processed_events = self.processed_events.borrow_mut();
                if !processed_events.contains(&event_key) {
                    processed_events.push_back(event_key);
                    return Some(event.clone());
                }
            }
        }
        
        None
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

    pub fn assert_fungible_transfer(&self, from: &Address, to: &Address, amount: i128) {
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

    pub fn assert_non_fungible_transfer(&self, from: &Address, to: &Address, token_id: TokenId) {
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

    pub fn assert_fungible_mint(&self, to: &Address, amount: i128) {
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

    pub fn assert_non_fungible_mint(&self, to: &Address, token_id: TokenId) {
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

    pub fn assert_fungible_burn(&self, from: &Address, amount: i128) {
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

    pub fn assert_non_fungible_burn(&self, from: &Address, token_id: TokenId) {
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

    pub fn assert_fungible_approve(
        &self,
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
        &self,
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
        &self,
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

    pub fn assert_consecutive_mint(&self, to: &Address, from_id: TokenId, to_id: TokenId) {
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

    pub fn refresh_events(&self) {
        *self.event_cache.borrow_mut() = Some(self.env.events().all());
        *self.processed_events.borrow_mut() = Vec::new(self.env);
    }

    pub fn assert_non_fungible_mints_ordered(&self, to: &Address, expected_ids: &[TokenId]) {
        for expected_id in expected_ids {
            self.assert_non_fungible_mint(to, *expected_id);
        }
    }

    pub fn assert_non_fungible_transfers_ordered(&self, from: &Address, to: &Address, expected_ids: &[TokenId]) {
        for expected_id in expected_ids {
            self.assert_non_fungible_transfer(from, to, *expected_id);
        }
    }
}