#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, String,
};

#[test]
fn test_reputation_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Reputation, ());
    let client = ReputationClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reporter = Address::generate(&env);
    let subject = Address::generate(&env);

    // Initialize
    client.initialize(&admin);

    // Cannot initialize twice
    let double_init = client.try_initialize(&admin);
    assert_eq!(double_init, Err(Ok(Error::AlreadyInitialized)));

    // Unauthorized reporter cannot record event
    let unauthorized_record = client.try_record_event(
        &reporter,
        &subject,
        &symbol_short!("milestone"),
        &15,
        &String::from_str(&env, "milestone-1"),
    );
    assert_eq!(unauthorized_record, Err(Ok(Error::Unauthorized)));

    // Add reporter
    client.add_reporter(&reporter);

    // Record positive event
    client.record_event(
        &reporter,
        &subject,
        &symbol_short!("milestone"),
        &15,
        &String::from_str(&env, "milestone-1"),
    );

    let evs = env.events().all();
    assert_eq!(evs.events().len(), 1);

    assert_eq!(client.get_score(&subject), 15);
    let badges = client.get_badges(&subject);
    assert_eq!(badges.len(), 1);
    assert_eq!(badges.get(0).unwrap().name, symbol_short!("Helper"));
    assert_eq!(badges.get(0).unwrap().tier, 1);

    // Record second event to reach tier 2 (50)
    client.record_event(
        &reporter,
        &subject,
        &symbol_short!("completed"),
        &40,
        &String::from_str(&env, "request-101"),
    );
    assert_eq!(client.get_score(&subject), 55);
    let badges2 = client.get_badges(&subject);
    assert_eq!(badges2.len(), 2);
    assert_eq!(badges2.get(1).unwrap().name, symbol_short!("Advocate"));

    // Remove reporter and verify they can no longer record
    client.remove_reporter(&reporter);
    let removed_record = client.try_record_event(
        &reporter,
        &subject,
        &symbol_short!("milestone"),
        &10,
        &String::from_str(&env, "milestone-2"),
    );
    assert_eq!(removed_record, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_config_update() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Reputation, ());
    let client = ReputationClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let current = client.get_config();
    assert_eq!(current.tier1_threshold, 10);

    client.set_config(&Config {
        tier1_threshold: 5,
        tier2_threshold: 25,
        tier3_threshold: 80,
    });

    let updated = client.get_config();
    assert_eq!(updated.tier1_threshold, 5);
    assert_eq!(updated.tier2_threshold, 25);
    assert_eq!(updated.tier3_threshold, 80);
}

#[test]
fn test_negative_deltas_and_badge_boundaries() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Reputation, ());
    let client = ReputationClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reporter = Address::generate(&env);
    let subject = Address::generate(&env);

    client.initialize(&admin);
    client.add_reporter(&reporter);

    // Initial score is 0, no badges
    assert_eq!(client.get_score(&subject), 0);
    assert_eq!(client.get_badges(&subject).len(), 0);

    // Score just below tier 1 threshold (9 < 10)
    client.record_event(
        &reporter,
        &subject,
        &symbol_short!("boost"),
        &9,
        &String::from_str(&env, "ref-1"),
    );
    assert_eq!(client.get_score(&subject), 9);
    assert_eq!(client.get_badges(&subject).len(), 0);

    // Hit exact tier 1 threshold (10)
    client.record_event(
        &reporter,
        &subject,
        &symbol_short!("boost"),
        &1,
        &String::from_str(&env, "ref-2"),
    );
    assert_eq!(client.get_score(&subject), 10);
    assert_eq!(client.get_badges(&subject).len(), 1);

    // Negative delta reduces score below threshold (10 - 2 = 8)
    client.record_event(
        &reporter,
        &subject,
        &symbol_short!("penalty"),
        &-2,
        &String::from_str(&env, "ref-3"),
    );
    assert_eq!(client.get_score(&subject), 8);
    assert_eq!(client.get_badges(&subject).len(), 0);

    // Zero delta is rejected with InvalidDelta
    let zero_delta = client.try_record_event(
        &reporter,
        &subject,
        &symbol_short!("noop"),
        &0,
        &String::from_str(&env, "ref-zero"),
    );
    assert_eq!(zero_delta, Err(Ok(Error::InvalidDelta)));
}
