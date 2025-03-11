use soroban_sdk::{symbol_short, testutils::Events, Address, Env, IntoVal, Symbol, Val, Vec};

pub struct EventAssertion<'a> {
    env: &'a Env,
    contract: Address,
}

impl<'a> EventAssertion<'a> {
    pub fn new(env: &'a Env, contract: Address) -> Self {
        Self { env, contract }
    }

    pub fn assert_burn(&self, from: &Address, token_id: u128) {
        let events = self.env.events().all();
        let burn_event = events.iter().find(|e| {
            let topics: Vec<Val> = e.1.clone();
            let topic_symbol: Symbol = topics.first().unwrap().into_val(self.env);
            topic_symbol == symbol_short!("burn")
        });

        assert!(burn_event.is_some(), "Burn event not found in event log");

        let (contract, topics, data) = burn_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "Burn event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("burn"));

        let event_from: Address = topics.get_unchecked(1).into_val(self.env);
        let event_token_id: u128 = data.into_val(self.env);

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

    pub fn assert_approve(
        &self,
        owner: &Address,
        spender: &Address,
        token_id: u128,
        live_until_ledger: u32,
    ) {
        let events = self.env.events().all();
        let approve_event = events.iter().find(|e| {
            let topics: Vec<Val> = e.1.clone();
            let topic_symbol: Symbol = topics.first().unwrap().into_val(self.env);
            topic_symbol == symbol_short!("approval")
        });

        assert!(approve_event.is_some(), "Approve event not found in event log");

        let (contract, topics, data) = approve_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 3, "Approve event should have 3 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, symbol_short!("approval"));

        let event_owner: Address = topics.get_unchecked(1).into_val(self.env);
        let event_token_id: u128 = topics.get_unchecked(2).into_val(self.env);
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
        approved: bool,
        live_until_ledger: u32,
    ) {
        let events = self.env.events().all();
        let approve_event = events.iter().find(|e| {
            let topics: Vec<Val> = e.1.clone();
            let topic_symbol: Symbol = topics.first().unwrap().into_val(self.env);
            topic_symbol == Symbol::new(self.env, "approval_for_all")
        });

        assert!(approve_event.is_some(), "ApproveForAll event not found in event log");

        let (contract, topics, data) = approve_event.unwrap();
        assert_eq!(contract, self.contract, "Event from wrong contract");

        let topics: Vec<Val> = topics.clone();
        assert_eq!(topics.len(), 2, "ApproveForAll event should have 2 topics");

        let topic_symbol: Symbol = topics.get_unchecked(0).into_val(self.env);
        assert_eq!(topic_symbol, Symbol::new(self.env, "approval_for_all"));

        let event_owner: Address = topics.get_unchecked(1).into_val(self.env);
        let event_data: (Address, bool, u32) = data.into_val(self.env);

        assert_eq!(&event_owner, owner, "Approve event has wrong owner address");
        assert_eq!(event_data.0, *operator, "Approve event has wrong operator address");
        assert_eq!(event_data.1, approved, "Approve event has wrong bool flag");
        assert_eq!(event_data.2, live_until_ledger, "Approve event has wrong live_until_ledger");
    }
}
