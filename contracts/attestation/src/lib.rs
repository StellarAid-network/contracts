#![no_std]

//! StellarAid Attestation contract.
//!
//! An admin-managed registry of attestors that can vouch for requests or users. Only
//! the stub (`initialize`) exists at this stage.

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has not been initialized.
    NotInitialized = 1,
    /// Initialization attempted twice.
    AlreadyInitialized = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Contract admin (instance storage).
    Admin,
}

const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

#[contract]
pub struct Attestation;

#[contractimpl]
impl Attestation {
    /// One-time setup. Stores the admin. Calling twice returns `AlreadyInitialized`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();

        let storage = env.storage().instance();
        if storage.has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        storage.set(&DataKey::Admin, &admin);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }
}

#[cfg(test)]
mod test;
