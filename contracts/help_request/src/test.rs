use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

fn config(env: &Env, admin: &Address) -> Config {
    Config {
        allowed_tokens: Vec::new(env),
        min_contribution: 1,
        approvers: Vec::from_array(env, [admin.clone()]),
        approval_threshold: 1,
        reputation_contract: None,
    }
}

#[test]
fn initialize_succeeds_once() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(HelpRequest, ());
    let client = HelpRequestClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    assert!(client.try_initialize(&admin, &config(&env, &admin)).is_ok());
}

#[test]
fn initialize_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(HelpRequest, ());
    let client = HelpRequestClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin, &config(&env, &admin));
    assert!(client
        .try_initialize(&admin, &config(&env, &admin))
        .is_err());
}
