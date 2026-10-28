use super::*;

#[test]
fn min_contribution_is_usable() {
    let amount = MIN_CONTRIBUTION;
    assert_eq!(amount.checked_add_amount(1), Some(2));
}

#[test]
fn checked_add_amount_reports_overflow() {
    assert_eq!(2i128.checked_add_amount(3), Some(5));
    assert_eq!(i128::MAX.checked_add_amount(1), None);
}

#[test]
fn timelocked_upgrade_and_versioned_migration_lifecycle() {
    use soroban_sdk::{
        contract, contractimpl,
        testutils::{Address as _, Ledger},
        Address, BytesN, Env,
    };

    #[contract]
    struct DummyContract;

    #[contractimpl]
    impl DummyContract {}

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(10_000);

    let contract_id = env.register(DummyContract, ());
    let admin = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[7u8; 32]);

    env.as_contract(&contract_id, || {
        init_storage_version(&env, STORAGE_VERSION_V1);
        assert_eq!(get_storage_version(&env), 1);
        assert_eq!(get_pending_upgrade(&env), None);

        // Propose upgrade with 1-day timelock (86,400s)
        let pending =
            propose_upgrade(&env, &admin, wasm_hash.clone(), DEFAULT_TIMELOCK_DELAY).unwrap();
        assert_eq!(pending.executable_after, 96_400);
        assert_eq!(get_pending_upgrade(&env), Some(pending));
    });

    // Executing before timelock expires fails
    env.ledger().set_timestamp(96_399);
    env.as_contract(&contract_id, || {
        assert_eq!(
            execute_upgrade(&env, &admin, false),
            Err(UpgradeError::TimelockNotExpired)
        );
    });

    // Executing at exact timelock boundary succeeds
    env.ledger().set_timestamp(96_400);
    env.as_contract(&contract_id, || {
        let applied = execute_upgrade(&env, &admin, false).unwrap();
        assert_eq!(applied, wasm_hash);
        assert_eq!(get_pending_upgrade(&env), None);
    });

    // Migrate storage v1 -> v2
    env.as_contract(&contract_id, || {
        let new_ver = migrate_storage(&env, &admin, 1, 2, |_e| {}).unwrap();
        assert_eq!(new_ver, 2);
        assert_eq!(get_storage_version(&env), 2);
    });

    // Re-running v1 -> v2 or skipping v2 -> v4 fails
    env.as_contract(&contract_id, || {
        assert_eq!(
            migrate_storage(&env, &admin, 1, 2, |_e| {}),
            Err(UpgradeError::InvalidVersionTransition)
        );
    });
    env.as_contract(&contract_id, || {
        assert_eq!(
            migrate_storage(&env, &admin, 2, 4, |_e| {}),
            Err(UpgradeError::InvalidVersionTransition)
        );
    });
}
