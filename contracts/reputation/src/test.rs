use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn initialize_succeeds_once() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Reputation, ());
    let client = ReputationClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    assert!(client.try_initialize(&admin).is_ok());
}

#[test]
fn initialize_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Reputation, ());
    let client = ReputationClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    assert!(client.try_initialize(&admin).is_err());
}
