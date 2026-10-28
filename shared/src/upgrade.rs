use soroban_sdk::{contracttype, Address, BytesN, Env};

/// Initial schema version assigned at contract initialization.
pub const STORAGE_VERSION_V1: u32 = 1;

/// Default time-lock delay for WASM upgrades (24 hours in ledger seconds).
pub const DEFAULT_TIMELOCK_DELAY: u64 = 86_400;

/// Instance storage keys reserved for versioned storage and time-locked upgrades.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpgradeKey {
    /// Current on-chain storage schema version (`u32`).
    StorageVersion,
    /// Queued WASM upgrade awaiting time-lock expiration.
    PendingUpgrade,
}

/// Metadata for a scheduled, time-locked contract WASM upgrade.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    /// Target WASM hash to install via `update_current_contract_wasm`.
    pub wasm_hash: BytesN<32>,
    /// Earliest ledger timestamp (seconds) at which the upgrade can be executed.
    pub executable_after: u64,
}

/// Errors returned by the time-locked upgrade and migration helpers.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UpgradeError {
    /// No upgrade has been proposed.
    NoPendingUpgrade,
    /// The time-lock delay has not yet elapsed.
    TimelockNotExpired,
    /// Arithmetic overflow when computing unlock timestamp.
    TimestampOverflow,
    /// Current storage version does not match expected `from_version`, or `to_version` is invalid.
    InvalidVersionTransition,
}

/// Initialize the contract's storage schema version in instance storage.
pub fn init_storage_version(env: &Env, version: u32) {
    env.storage()
        .instance()
        .set(&UpgradeKey::StorageVersion, &version);
}

/// Read the current storage schema version (defaults to `STORAGE_VERSION_V1`).
pub fn get_storage_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&UpgradeKey::StorageVersion)
        .unwrap_or(STORAGE_VERSION_V1)
}

/// Read the currently queued upgrade proposal, if any.
pub fn get_pending_upgrade(env: &Env) -> Option<PendingUpgrade> {
    env.storage().instance().get(&UpgradeKey::PendingUpgrade)
}

/// Propose a time-locked WASM upgrade.
///
/// Requires authorization from `admin` (intended to be a multisig account).
/// Records `PendingUpgrade` with `executable_after = now + timelock_delay`.
pub fn propose_upgrade(
    env: &Env,
    admin: &Address,
    wasm_hash: BytesN<32>,
    timelock_delay: u64,
) -> Result<PendingUpgrade, UpgradeError> {
    admin.require_auth();

    let now = env.ledger().timestamp();
    let executable_after = now
        .checked_add(timelock_delay)
        .ok_or(UpgradeError::TimestampOverflow)?;

    let pending = PendingUpgrade {
        wasm_hash,
        executable_after,
    };

    env.storage()
        .instance()
        .set(&UpgradeKey::PendingUpgrade, &pending);

    Ok(pending)
}

/// Cancel a pending WASM upgrade before it is executed.
///
/// Requires authorization from `admin`.
pub fn cancel_upgrade(env: &Env, admin: &Address) -> Result<(), UpgradeError> {
    admin.require_auth();

    if !env.storage().instance().has(&UpgradeKey::PendingUpgrade) {
        return Err(UpgradeError::NoPendingUpgrade);
    }

    env.storage().instance().remove(&UpgradeKey::PendingUpgrade);
    Ok(())
}

/// Validate that the pending upgrade's time-lock has expired and clear the proposal.
///
/// When `apply_wasm` is true, also calls `env.deployer().update_current_contract_wasm`.
pub fn execute_upgrade(
    env: &Env,
    admin: &Address,
    apply_wasm: bool,
) -> Result<BytesN<32>, UpgradeError> {
    admin.require_auth();

    let pending: PendingUpgrade = env
        .storage()
        .instance()
        .get(&UpgradeKey::PendingUpgrade)
        .ok_or(UpgradeError::NoPendingUpgrade)?;

    let now = env.ledger().timestamp();
    if now < pending.executable_after {
        return Err(UpgradeError::TimelockNotExpired);
    }

    env.storage().instance().remove(&UpgradeKey::PendingUpgrade);

    if apply_wasm {
        #[allow(deprecated)]
        env.deployer()
            .update_current_contract_wasm(pending.wasm_hash.clone());
    }

    Ok(pending.wasm_hash)
}

/// Execute an explicit, sequential storage migration from `from_version` to `to_version`.
///
/// Requires `admin` authorization, verifies `current == from_version` and
/// `to_version == from_version + 1`, runs `migrate_fn`, and bumps `StorageVersion`.
pub fn migrate_storage<F>(
    env: &Env,
    admin: &Address,
    from_version: u32,
    to_version: u32,
    migrate_fn: F,
) -> Result<u32, UpgradeError>
where
    F: FnOnce(&Env),
{
    admin.require_auth();

    let current = get_storage_version(env);
    let expected_next = from_version
        .checked_add(1)
        .ok_or(UpgradeError::InvalidVersionTransition)?;

    if current != from_version || to_version != expected_next {
        return Err(UpgradeError::InvalidVersionTransition);
    }

    migrate_fn(env);

    env.storage()
        .instance()
        .set(&UpgradeKey::StorageVersion, &to_version);

    Ok(to_version)
}
