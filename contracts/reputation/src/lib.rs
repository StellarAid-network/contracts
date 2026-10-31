#![no_std]

//! StellarAid Reputation contract.
//!
//! Authorized reporter contracts apply score changes to addresses based on verified outcomes.
//! Badges are awarded at configurable thresholds and are read-only for observers.

pub mod errors;
pub mod events;
pub mod types;

pub use crate::errors::Error;
pub use crate::types::{Badge, Config, DataKey};

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};

const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

const PERSISTENT_TTL_THRESHOLD: u32 = 120_960;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_036_800;

const DEFAULT_TIER1_THRESHOLD: i64 = 10;
const DEFAULT_TIER2_THRESHOLD: i64 = 50;
const DEFAULT_TIER3_THRESHOLD: i64 = 100;

/// Soroban smart contract recording outcome-driven reputation scores and threshold badges.
#[contract]
pub struct Reputation;

#[contractimpl]
impl Reputation {
    /// One-time setup. Stores the admin and default badge threshold configuration.
    /// Calling twice returns `AlreadyInitialized`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();

        let storage = env.storage().instance();
        if storage.has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        storage.set(&DataKey::Admin, &admin);
        storage.set(
            &DataKey::Config,
            &Config {
                tier1_threshold: DEFAULT_TIER1_THRESHOLD,
                tier2_threshold: DEFAULT_TIER2_THRESHOLD,
                tier3_threshold: DEFAULT_TIER3_THRESHOLD,
            },
        );
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }

    /// Update badge threshold config. Admin-only.
    pub fn set_config(env: Env, config: Config) -> Result<(), Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        admin.require_auth();

        storage.set(&DataKey::Config, &config);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }

    /// Read badge threshold config.
    pub fn get_config(env: Env) -> Result<Config, Error> {
        let storage = env.storage().instance();
        let cfg: Config = storage.get(&DataKey::Config).ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
        Ok(cfg)
    }

    /// Add an authorized reporter contract. Admin-only.
    pub fn add_reporter(env: Env, reporter: Address) -> Result<(), Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        admin.require_auth();
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        env.storage()
            .persistent()
            .set(&DataKey::Reporter(reporter.clone()), &true);
        env.storage().persistent().extend_ttl(
            &DataKey::Reporter(reporter),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        Ok(())
    }

    /// Remove an authorized reporter contract. Admin-only.
    pub fn remove_reporter(env: Env, reporter: Address) -> Result<(), Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        admin.require_auth();
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        env.storage()
            .persistent()
            .remove(&DataKey::Reporter(reporter));

        Ok(())
    }

    /// Apply a score change tied to a reference ID.
    /// Only authorized reporters can call this. Emits `score_changed`.
    pub fn record_event(
        env: Env,
        reporter: Address,
        subject: Address,
        _kind: Symbol,
        delta: i32,
        ref_id: String,
    ) -> Result<(), Error> {
        reporter.require_auth();

        let storage = env.storage().instance();
        storage
            .get::<_, Address>(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        if delta == 0 {
            return Err(Error::InvalidDelta);
        }

        let is_reporter: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Reporter(reporter.clone()))
            .unwrap_or(false);

        if !is_reporter {
            return Err(Error::Unauthorized);
        }

        env.storage().persistent().extend_ttl(
            &DataKey::Reporter(reporter),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        let current_score: i64 = env
            .storage()
            .persistent()
            .get(&DataKey::Score(subject.clone()))
            .unwrap_or(0i64);

        let new_score = current_score
            .checked_add(delta as i64)
            .ok_or(Error::InvalidDelta)?;

        env.storage()
            .persistent()
            .set(&DataKey::Score(subject.clone()), &new_score);
        env.storage().persistent().extend_ttl(
            &DataKey::Score(subject.clone()),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::score_changed(&env, subject, delta, new_score, ref_id);

        Ok(())
    }

    /// Read current reputation score for a subject. Defaults to 0 if none recorded.
    pub fn get_score(env: Env, subject: Address) -> i64 {
        let key = DataKey::Score(subject);
        let score: Option<i64> = env.storage().persistent().get(&key);
        if let Some(s) = score {
            env.storage().persistent().extend_ttl(
                &key,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
            s
        } else {
            0i64
        }
    }

    /// Read earned badges for a subject based on current score and configured thresholds.
    pub fn get_badges(env: Env, subject: Address) -> Vec<Badge> {
        let mut badges = Vec::new(&env);
        let config = match env.storage().instance().get::<_, Config>(&DataKey::Config) {
            Some(c) => c,
            None => return badges,
        };

        let score = Self::get_score(env.clone(), subject);

        if score >= config.tier1_threshold {
            badges.push_back(Badge {
                name: symbol_short!("Helper"),
                tier: 1,
                threshold: config.tier1_threshold,
            });
        }
        if score >= config.tier2_threshold {
            badges.push_back(Badge {
                name: symbol_short!("Advocate"),
                tier: 2,
                threshold: config.tier2_threshold,
            });
        }
        if score >= config.tier3_threshold {
            badges.push_back(Badge {
                name: symbol_short!("Champion"),
                tier: 3,
                threshold: config.tier3_threshold,
            });
        }

        badges
    }
}

#[cfg(test)]
mod test;
