extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, Vec};

use crate::rwa::claim_topics_and_issuers::storage::{
    add_claim_topic, add_trusted_issuer, get_claim_topic_issuers, get_claim_topics,
    get_claim_topics_and_issuers, get_trusted_issuer_claim_topics, get_trusted_issuers,
    has_claim_topic, is_trusted_issuer, remove_claim_topic, remove_trusted_issuer,
    update_issuer_claim_topics,
};

#[contract]
struct MockContract;

#[test]
fn add_claim_topic_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Check initial state
        let topics = get_claim_topics(&e);
        assert!(topics.is_empty());

        // Add claim topic
        add_claim_topic(&e, 1);

        // Verify topic was added
        let topics = get_claim_topics(&e);
        assert_eq!(topics.len(), 1);
        assert!(topics.contains(1));
    });
}

#[test]
fn add_multiple_claim_topics_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Add multiple claim topics
        add_claim_topic(&e, 1); // KYC
        add_claim_topic(&e, 2); // AML
        add_claim_topic(&e, 3); // Accreditation

        // Verify all topics were added
        let topics = get_claim_topics(&e);
        assert_eq!(topics.len(), 3);
        assert!(topics.contains(1));
        assert!(topics.contains(2));
        assert!(topics.contains(3));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #372)")]
fn add_duplicate_claim_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Add claim topic
        add_claim_topic(&e, 1);

        // Try to add the same topic again
        add_claim_topic(&e, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #374)")]
fn add_claim_topic_exceeds_limit_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Add 15 topics (the limit)
        for i in 1..=15 {
            add_claim_topic(&e, i);
        }

        // Try to add the 16th topic (should panic)
        add_claim_topic(&e, 16);
    });
}

#[test]
fn remove_claim_topic_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Add claim topics
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);

        // Remove one topic
        remove_claim_topic(&e, 1);

        // Verify topic was removed
        let topics = get_claim_topics(&e);
        assert_eq!(topics.len(), 1);
        assert!(!topics.contains(1));
        assert!(topics.contains(2));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #370)")]
fn remove_nonexistent_claim_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Add one topic
        add_claim_topic(&e, 1);

        // Try to remove a non-existent topic
        remove_claim_topic(&e, 999);
    });
}

// ################## TRUSTED ISSUERS TESTS ##################

#[test]
fn add_trusted_issuer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);

        // Create claim topics vector
        let mut claim_topics = Vec::new(&e);
        claim_topics.push_back(1);
        claim_topics.push_back(2);

        // Check initial state
        assert!(!is_trusted_issuer(&e, &issuer));
        let issuers = get_trusted_issuers(&e);
        assert!(issuers.is_empty());

        // Add trusted issuer
        add_trusted_issuer(&e, &issuer, &claim_topics);

        // Verify issuer was added
        assert!(is_trusted_issuer(&e, &issuer));
        let issuers = get_trusted_issuers(&e);
        assert_eq!(issuers.len(), 1);
        assert!(issuers.contains(&issuer));

        // Verify issuer's claim topics
        let issuer_topics = get_trusted_issuer_claim_topics(&e, &issuer);
        assert_eq!(issuer_topics.len(), 2);
        assert!(issuer_topics.contains(1));
        assert!(issuer_topics.contains(2));
    });
}

#[test]
fn add_multiple_trusted_issuers_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);
        add_claim_topic(&e, 3);

        // Create different claim topics for each issuer
        let mut topics1 = Vec::new(&e);
        topics1.push_back(1);

        let mut topics2 = Vec::new(&e);
        topics2.push_back(2);
        topics2.push_back(3);

        // Add both issuers
        add_trusted_issuer(&e, &issuer1, &topics1);
        add_trusted_issuer(&e, &issuer2, &topics2);

        // Verify both issuers are trusted
        assert!(is_trusted_issuer(&e, &issuer1));
        assert!(is_trusted_issuer(&e, &issuer2));

        let issuers = get_trusted_issuers(&e);
        assert_eq!(issuers.len(), 2);
        assert!(issuers.contains(&issuer1));
        assert!(issuers.contains(&issuer2));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #376)")]
fn add_trusted_issuer_empty_topics_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        let empty_topics = Vec::new(&e);
        add_trusted_issuer(&e, &issuer, &empty_topics);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #374)")]
fn add_trusted_issuer_too_many_topics_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        // Create 16 topics (exceeds limit)
        let mut topics = Vec::new(&e);
        for i in 1..=16 {
            topics.push_back(i);
        }

        add_trusted_issuer(&e, &issuer, &topics);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #373)")]
fn add_duplicate_trusted_issuer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Add issuer first time
        add_trusted_issuer(&e, &issuer, &topics);

        // Try to add the same issuer again
        add_trusted_issuer(&e, &issuer, &topics);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #375)")]
fn add_trusted_issuer_exceeds_limit_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Add 50 issuers (the limit)
        for _ in 0..50 {
            let issuer = Address::generate(&e);
            add_trusted_issuer(&e, &issuer, &topics);
        }

        // Try to add the 51st issuer (should panic)
        let issuer = Address::generate(&e);
        add_trusted_issuer(&e, &issuer, &topics);
    });
}

#[test]
fn remove_trusted_issuer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Add both issuers
        add_trusted_issuer(&e, &issuer1, &topics);
        add_trusted_issuer(&e, &issuer2, &topics);

        // Remove one issuer
        remove_trusted_issuer(&e, &issuer1);

        // // Verify issuer was removed
        assert!(!is_trusted_issuer(&e, &issuer1));
        assert!(is_trusted_issuer(&e, &issuer2));

        let issuers = get_trusted_issuers(&e);
        assert_eq!(issuers.len(), 1);
        assert!(!issuers.contains(&issuer1));
        assert!(issuers.contains(&issuer2));

        // Verify issuer's claim topics were removed
        let issuer_topics = get_claim_topic_issuers(&e, 1);
        assert_eq!(issuer_topics.len(), 1);
        assert!(!issuer_topics.contains(&issuer1));
        assert!(issuer_topics.contains(&issuer2));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #371)")]
fn remove_nonexistent_trusted_issuer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Add one issuer
        add_trusted_issuer(&e, &issuer1, &topics);

        // Try to remove a non-existent issuer
        remove_trusted_issuer(&e, &issuer2);
    });
}

#[test]
fn get_claim_topic_issuers_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);
    let issuer3 = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);
        add_claim_topic(&e, 3);

        // Create different topic combinations
        let mut topics1 = Vec::new(&e);
        topics1.push_back(1);
        topics1.push_back(2);

        let mut topics2 = Vec::new(&e);
        topics2.push_back(1);
        topics2.push_back(3);

        let mut topics3 = Vec::new(&e);
        topics3.push_back(2);

        // Add issuers
        add_trusted_issuer(&e, &issuer1, &topics1);
        add_trusted_issuer(&e, &issuer2, &topics2);
        add_trusted_issuer(&e, &issuer3, &topics3);

        // Check topic 1 issuers (should have issuer1 and issuer2)
        let topic1_issuers = get_claim_topic_issuers(&e, 1);
        assert_eq!(topic1_issuers.len(), 2);
        assert!(topic1_issuers.contains(&issuer1));
        assert!(topic1_issuers.contains(&issuer2));
        assert!(!topic1_issuers.contains(&issuer3));

        // // Check topic 2 issuers (should have issuer1 and issuer3)
        let topic2_issuers = get_claim_topic_issuers(&e, 2);
        assert_eq!(topic2_issuers.len(), 2);
        assert!(topic2_issuers.contains(&issuer1));
        assert!(!topic2_issuers.contains(&issuer2));
        assert!(topic2_issuers.contains(&issuer3));

        // // Check topic 3 issuers (should have only issuer2)
        let topic3_issuers = get_claim_topic_issuers(&e, 3);
        assert_eq!(topic3_issuers.len(), 1);
        assert!(!topic3_issuers.contains(&issuer1));
        assert!(topic3_issuers.contains(&issuer2));
        assert!(!topic3_issuers.contains(&issuer3));
    });
}

#[test]
fn has_claim_topic_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);
        add_claim_topic(&e, 3);

        let mut topics = Vec::new(&e);
        topics.push_back(1);
        topics.push_back(3);

        // Add issuer with specific topics
        add_trusted_issuer(&e, &issuer, &topics);

        // Check topics the issuer has
        assert!(has_claim_topic(&e, &issuer, 1));
        assert!(has_claim_topic(&e, &issuer, 3));

        // Check topics the issuer doesn't have
        assert!(!has_claim_topic(&e, &issuer, 2));
        assert!(!has_claim_topic(&e, &issuer, 999));
    });
}

#[test]
fn update_issuer_claim_topics_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);
        add_claim_topic(&e, 3);
        add_claim_topic(&e, 4);
        add_claim_topic(&e, 5);

        // Initial topics
        let mut initial_topics = Vec::new(&e);
        initial_topics.push_back(1);
        initial_topics.push_back(2);
        initial_topics.push_back(3);

        // Add issuer
        add_trusted_issuer(&e, &issuer, &initial_topics);

        // New topics (some overlap, some new, some removed)
        let mut new_topics = Vec::new(&e);
        new_topics.push_back(2); // kept
        new_topics.push_back(3); // kept
        new_topics.push_back(4); // new
        new_topics.push_back(5); // new
                                 // topic 1 is removed

        // Update issuer's claim topics
        update_issuer_claim_topics(&e, &issuer, &new_topics);

        // Verify updated topics
        let issuer_topics = get_trusted_issuer_claim_topics(&e, &issuer);
        assert_eq!(issuer_topics.len(), 4);
        assert!(!issuer_topics.contains(1)); // removed
        assert!(issuer_topics.contains(2)); // kept
        assert!(issuer_topics.contains(3)); // kept
        assert!(issuer_topics.contains(4)); // new
        assert!(issuer_topics.contains(5)); // new

        // Verify reverse mappings are updated
        let topic1_issuers = get_claim_topic_issuers(&e, 1);
        assert!(!topic1_issuers.contains(&issuer)); // should be removed

        let topic4_issuers = get_claim_topic_issuers(&e, 4);
        assert!(topic4_issuers.contains(&issuer)); // should be added
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #376)")]
fn update_issuer_claim_topics_empty_topics_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Add issuer first
        add_trusted_issuer(&e, &issuer, &topics);

        // Try to update with empty topics
        let empty_topics = Vec::new(&e);
        update_issuer_claim_topics(&e, &issuer, &empty_topics);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #374)")]
fn update_issuer_claim_topics_too_many_topics_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Add issuer first
        add_trusted_issuer(&e, &issuer, &topics);

        // Try to update with too many topics
        let mut too_many_topics = Vec::new(&e);
        for i in 1..=16 {
            too_many_topics.push_back(i);
        }
        update_issuer_claim_topics(&e, &issuer, &too_many_topics);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #371)")]
fn update_nonexistent_issuer_claim_topics_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer = Address::generate(&e);

    e.as_contract(&address, || {
        add_claim_topic(&e, 1);

        let mut topics = Vec::new(&e);
        topics.push_back(1);

        // Try to update a non-existent issuer
        update_issuer_claim_topics(&e, &issuer, &topics);
    });
}

#[test]
fn complex_scenario_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);
    let issuer3 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add claim topics
        add_claim_topic(&e, 1);
        add_claim_topic(&e, 2);
        add_claim_topic(&e, 3);

        // Add issuers with different capabilities
        let mut kyc_aml = Vec::new(&e);
        kyc_aml.push_back(1);
        kyc_aml.push_back(2);
        add_trusted_issuer(&e, &issuer1, &kyc_aml);

        let mut accreditation_only = Vec::new(&e);
        accreditation_only.push_back(3);
        add_trusted_issuer(&e, &issuer2, &accreditation_only);

        let mut all_topics = Vec::new(&e);
        all_topics.push_back(1);
        all_topics.push_back(2);
        all_topics.push_back(3);
        add_trusted_issuer(&e, &issuer3, &all_topics);

        // Verify complex queries
        assert_eq!(get_claim_topics(&e).len(), 3);
        assert_eq!(get_trusted_issuers(&e).len(), 3);

        // Check KYC issuers
        let kyc_issuers = get_claim_topic_issuers(&e, 1);
        assert_eq!(kyc_issuers.len(), 2);
        assert!(kyc_issuers.contains(&issuer1));
        assert!(kyc_issuers.contains(&issuer3));

        // Check accreditation issuers
        let accred_issuers = get_claim_topic_issuers(&e, 3);
        assert_eq!(accred_issuers.len(), 2);
        assert!(accred_issuers.contains(&issuer2));
        assert!(accred_issuers.contains(&issuer3));

        // Update issuer1 to only do KYC
        let mut kyc_only = Vec::new(&e);
        kyc_only.push_back(1);
        update_issuer_claim_topics(&e, &issuer1, &kyc_only);

        // Verify AML issuers updated
        let aml_issuers = get_claim_topic_issuers(&e, 2);
        assert_eq!(aml_issuers.len(), 1);
        assert!(!aml_issuers.contains(&issuer1));
        assert!(aml_issuers.contains(&issuer3));

        // Remove issuer2
        remove_trusted_issuer(&e, &issuer2);

        // Verify accreditation issuers updated
        let accred_issuers = get_claim_topic_issuers(&e, 3);
        assert_eq!(accred_issuers.len(), 1);
        assert!(!accred_issuers.contains(&issuer2));
        assert!(accred_issuers.contains(&issuer3));

        // Final state check
        assert_eq!(get_trusted_issuers(&e).len(), 2);
    });
}

#[test]
fn get_claim_topics_and_issuers_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);
    let issuer3 = Address::generate(&e);

    e.as_contract(&address, || {
        // Test empty state
        let result = get_claim_topics_and_issuers(&e);
        assert!(result.is_empty());

        // Add claim topics
        add_claim_topic(&e, 1); // KYC
        add_claim_topic(&e, 2); // AML
        add_claim_topic(&e, 3); // Accreditation

        // Test with topics but no issuers
        let result = get_claim_topics_and_issuers(&e);
        assert_eq!(result.len(), 3);
        assert!(result.get(1).unwrap().is_empty());
        assert!(result.get(2).unwrap().is_empty());
        assert!(result.get(3).unwrap().is_empty());

        // Add issuers with different topic combinations
        let mut kyc_aml = Vec::new(&e);
        kyc_aml.push_back(1);
        kyc_aml.push_back(2);
        add_trusted_issuer(&e, &issuer1, &kyc_aml);

        let mut accreditation_only = Vec::new(&e);
        accreditation_only.push_back(3);
        add_trusted_issuer(&e, &issuer2, &accreditation_only);

        let mut all_topics = Vec::new(&e);
        all_topics.push_back(1);
        all_topics.push_back(2);
        all_topics.push_back(3);
        add_trusted_issuer(&e, &issuer3, &all_topics);

        // Test complete mapping
        let result = get_claim_topics_and_issuers(&e);
        assert_eq!(result.len(), 3);

        // Verify topic 1 (KYC) has only issuer1 and issuer3
        let topic1_issuers = result.get(1).unwrap();
        assert_eq!(topic1_issuers.len(), 2);
        assert!(topic1_issuers.contains(&issuer1));
        assert!(topic1_issuers.contains(&issuer3));
        assert!(!topic1_issuers.contains(&issuer2));

        // Verify topic 2 (AML) has only issuer1 and issuer3
        let topic2_issuers = result.get(2).unwrap();
        assert_eq!(topic2_issuers.len(), 2);
        assert!(topic2_issuers.contains(&issuer1));
        assert!(topic2_issuers.contains(&issuer3));
        assert!(!topic2_issuers.contains(&issuer2));

        // Verify topic 3 (Accreditation) has only issuer2 and issuer3
        let topic3_issuers = result.get(3).unwrap();
        assert_eq!(topic3_issuers.len(), 2);
        assert!(!topic3_issuers.contains(&issuer1));
        assert!(topic3_issuers.contains(&issuer2));
        assert!(topic3_issuers.contains(&issuer3));

        // Update issuer1 to only handle KYC
        let mut kyc_only = Vec::new(&e);
        kyc_only.push_back(1);
        update_issuer_claim_topics(&e, &issuer1, &kyc_only);

        // Verify updated mapping
        let result = get_claim_topics_and_issuers(&e);

        // Topic 1 should still have both issuers
        let topic1_issuers = result.get(1).unwrap();
        assert_eq!(topic1_issuers.len(), 2);
        assert!(topic1_issuers.contains(&issuer1));
        assert!(topic1_issuers.contains(&issuer3));

        // Topic 2 should now only have issuer3
        let topic2_issuers = result.get(2).unwrap();
        assert_eq!(topic2_issuers.len(), 1);
        assert!(!topic2_issuers.contains(&issuer1));
        assert!(topic2_issuers.contains(&issuer3));

        // Remove issuer2
        remove_trusted_issuer(&e, &issuer2);

        // Verify final mapping
        let result = get_claim_topics_and_issuers(&e);

        // Topic 3 should now only have issuer3
        let topic3_issuers = result.get(3).unwrap();
        assert_eq!(topic3_issuers.len(), 1);
        assert!(!topic3_issuers.contains(&issuer2));
        assert!(topic3_issuers.contains(&issuer3));
    });
}

#[test]
fn get_claim_topics_and_issuers_empty_state_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Test completely empty state
        let result = get_claim_topics_and_issuers(&e);
        assert!(result.is_empty());
    });
}

#[test]
fn get_claim_topics_and_issuers_single_topic_multiple_issuers_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);
    let issuer3 = Address::generate(&e);

    e.as_contract(&address, || {
        // Add single claim topic
        add_claim_topic(&e, 1);

        // Add multiple issuers for the same topic
        let mut topic1 = Vec::new(&e);
        topic1.push_back(1);

        add_trusted_issuer(&e, &issuer1, &topic1);
        add_trusted_issuer(&e, &issuer2, &topic1);
        add_trusted_issuer(&e, &issuer3, &topic1);

        // Verify mapping
        let result = get_claim_topics_and_issuers(&e);
        assert_eq!(result.len(), 1);

        let topic1_issuers = result.get(1).unwrap();
        assert_eq!(topic1_issuers.len(), 3);
        assert!(topic1_issuers.contains(&issuer1));
        assert!(topic1_issuers.contains(&issuer2));
        assert!(topic1_issuers.contains(&issuer3));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #370)")]
fn get_claim_topic_issuers_nonexistent_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Try to get issuers for a claim topic that doesn't exist
        get_claim_topic_issuers(&e, 999);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #371)")]
fn get_trusted_issuer_claim_topics_nonexistent_issuer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let nonexistent_issuer = Address::generate(&e);

    e.as_contract(&address, || {
        // Try to get claim topics for an issuer that doesn't exist
        get_trusted_issuer_claim_topics(&e, &nonexistent_issuer);
    });
}
