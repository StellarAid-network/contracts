#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, BytesN, Env,
};

#[test]
fn test_attestation_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AttestationContract, ());
    let client = AttestationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let subject = Address::generate(&env);

    client.initialize(&admin);

    // Cannot initialize twice
    let double_init = client.try_initialize(&admin);
    assert_eq!(double_init, Err(Ok(Error::AlreadyInitialized)));

    let meta_hash = BytesN::from_array(&env, &[1u8; 32]);
    let content_hash = BytesN::from_array(&env, &[2u8; 32]);

    // Unapproved cannot attest
    let unauth = client.try_attest(
        &attestor,
        &subject,
        &symbol_short!("need"),
        &content_hash,
        &None,
    );
    assert_eq!(unauth, Err(Ok(Error::Unauthorized)));

    // Admin approves attestor
    client.add_attestor(&attestor, &meta_hash);

    // Cannot attest with past expiry
    env.ledger().set_timestamp(1_000);
    let past_exp = client.try_attest(
        &attestor,
        &subject,
        &symbol_short!("need"),
        &content_hash,
        &Some(999),
    );
    assert_eq!(past_exp, Err(Ok(Error::InvalidExpiry)));

    // Create valid attestation with expiry at 2_000
    let id1 = client.attest(
        &attestor,
        &subject,
        &symbol_short!("need"),
        &content_hash,
        &Some(2_000),
    );
    assert_eq!(id1, 1);

    // Verify event
    let evs = env.events().all();
    assert_eq!(evs.events().len(), 1);

    // Create second permanent attestation (no expiry)
    let content_hash2 = BytesN::from_array(&env, &[3u8; 32]);
    let id2 = client.attest(
        &attestor,
        &subject,
        &symbol_short!("id"),
        &content_hash2,
        &None,
    );
    assert_eq!(id2, 2);

    let list = client.get_attestations(&subject);
    assert_eq!(list.len(), 2);

    // Advance time past first attestation's expiry
    env.ledger().set_timestamp(2_500);
    let list_after_expiry = client.get_attestations(&subject);
    assert_eq!(list_after_expiry.len(), 1);
    assert_eq!(list_after_expiry.get(0).unwrap().id, 2);

    // Revoke the second attestation
    client.revoke(&attestor, &id2);

    // Double revoke fails
    let double_revoke = client.try_revoke(&attestor, &id2);
    assert_eq!(double_revoke, Err(Ok(Error::AlreadyRevoked)));

    // Non-attestor cannot revoke
    let other = Address::generate(&env);
    let unauth_revoke = client.try_revoke(&other, &id1);
    assert_eq!(unauth_revoke, Err(Ok(Error::Unauthorized)));

    // Now subject has 0 active attestations
    let final_list = client.get_attestations(&subject);
    assert_eq!(final_list.len(), 0);

    // Remove attestor and verify they can no longer attest
    client.remove_attestor(&attestor);
    let removed_attest = client.try_attest(
        &attestor,
        &subject,
        &symbol_short!("need"),
        &content_hash,
        &None,
    );
    assert_eq!(removed_attest, Err(Ok(Error::Unauthorized)));
}
