#![no_std]

//! StellarAid Help Request contract.
//!
//! Funding-goal escrow with a deadline, refunds, and milestone release. Only the
//! scaffolding exists at this stage: types, errors, event helpers, and `initialize`.

mod errors;
pub mod events;
mod types;

pub use crate::errors::Error;
pub use crate::types::{Config, DataKey, Milestone, Request, Status};

use soroban_sdk::{contract, contractimpl, Address, Env};

/// Minimum instance-storage TTL kept on reads/writes, in ledgers (~1 day at 5s).
const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
/// Target instance-storage TTL when extending (~30 days at 5s).
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

#[contract]
pub struct HelpRequest;

#[contractimpl]
impl HelpRequest {
    /// One-time setup. Stores the admin and configuration in instance storage.
    ///
    /// `admin` must authorize the call. Calling twice returns `AlreadyInitialized`.
    pub fn initialize(env: Env, admin: Address, config: Config) -> Result<(), Error> {
        admin.require_auth();

        let storage = env.storage().instance();
        if storage.has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        storage.set(&DataKey::Admin, &admin);
        storage.set(&DataKey::Config, &config);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }
}

#[cfg(test)]
mod test;
